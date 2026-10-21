"""Fast lane: pure temporary artifacts and a fake Cargo; no native build.

The slow boundary lane is the real Cargo/native CTest run, owned by CMake.
Cargo record shapes were calibrated against retained native build logs 120/127.
"""
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import runpy
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / 'scripts/prepare-trigger-clap-native.py'


class TriggerNativeFixtureTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location('trigger_native_fixture', DRIVER)
        cls.driver = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.driver)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.manifest = self.root / 'adapter/Cargo.toml'
        self.manifest.parent.mkdir()
        self.manifest.write_text('[package]\nname="fixture"\n')
        self.target = self.root / 'cargo target'
        self.plugin = self.target / 'release/deps/libdandrum_clap_adapter.so'
        self.plugin.parent.mkdir(parents=True)
        self.plugin.write_bytes(b'actual deps cdylib\x00\xff')
        self.plugin.chmod(0o754)
        self.stale = self.target / 'release/libdandrum_clap_adapter.so'
        self.stale.write_bytes(b'STALE convenience library')
        self.executable = self.plugin.parent / 'trigger_gui-38f819efb15c476e'
        self.executable.write_bytes(b'actual test executable\x00')
        self.executable.chmod(0o751)
        self.resources = self.root / 'DandrumTrigger.resources'
        (self.resources / 'nested').mkdir(parents=True)
        (self.resources / 'nested/ui.bin').write_bytes(b'UI resource\x00\xff')
        self.output = self.root / 'fixture'
        self.scenario = self.root / 'cargo-output.json'
        self.invocation = self.root / 'invocation.json'
        self.cargo = self.root / 'fake cargo'
        self.cargo.write_text(
            f'#!{sys.executable}\n'
            'import json, os, pathlib, sys\n'
            f'p = pathlib.Path({str(self.root)!r})\n'
            'invocation = p / "invocation.json"\n'
            'assert not invocation.exists(), "Cargo invoked more than once"\n'
            'invocation.write_text(json.dumps({"argv": sys.argv, "cwd": os.getcwd(), '
            '"target_dir": os.environ.get("CARGO_TARGET_DIR"), '
            '"offline": os.environ.get("CARGO_NET_OFFLINE")}))\n'
            'scenario = json.loads((p / "cargo-output.json").read_text())\n'
            'sys.stdout.buffer.write(scenario["stdout"].encode())\n'
            'sys.stderr.buffer.write(scenario["stderr"].encode())\n'
            'sys.exit(scenario["status"])\n')
        self.cargo.chmod(0o755)
        # Shape copied from the real normal-library compiler-artifact record;
        # every path and artifact byte belongs to this temporary fixture.
        self.library_record = {
            'reason': 'compiler-artifact',
            'package_id': f'path+file://{self.manifest.parent}#dandrum_clap_adapter@0.1.0',
            'manifest_path': str(self.manifest),
            'target': {'kind': ['cdylib', 'rlib'], 'crate_types': ['cdylib', 'rlib'],
                       'name': 'dandrum_clap_adapter', 'src_path': str(self.manifest.parent / 'src/lib.rs'),
                       'edition': '2024', 'doc': True, 'doctest': True, 'test': True},
            'profile': {'opt_level': '3', 'debuginfo': 0, 'debug_assertions': False,
                        'overflow_checks': False, 'test': False},
            'features': ['default', 'trigger', 'trigger-editor'],
            'filenames': [str(self.plugin), str(self.plugin.with_suffix('.rlib'))],
            'executable': None, 'fresh': False,
        }
        self.test_record = copy.deepcopy(self.library_record)
        self.test_record.update(
            target={'kind': ['test'], 'crate_types': ['bin'], 'name': 'trigger_gui',
                    'src_path': str(self.manifest.parent / 'tests/trigger_gui.rs'),
                    'edition': '2024', 'doc': False, 'doctest': False, 'test': True},
            profile={**self.library_record['profile'], 'test': True},
            filenames=[str(self.executable)], executable=str(self.executable))
        self.records = [self.library_record, self.test_record]

    def run_driver(self, records=None, status=0, cli=False, extra_args=()):
        stdout = 'Cargo non-JSON progress\n' + '\n'.join(
            json.dumps(record) for record in (self.records if records is None else records)) + '\n'
        self.scenario.write_text(json.dumps({'stdout': stdout, 'stderr': 'retained cargo stderr\n',
                                             'status': status}))
        self.invocation.unlink(missing_ok=True)
        stderr = io.StringIO()
        with patch.dict(os.environ, {'CARGO_NET_OFFLINE': 'true'}), contextlib.redirect_stderr(stderr):
            argv = [
                '--cargo', str(self.cargo), '--manifest', str(self.manifest),
                '--target-dir', str(self.target), '--resource-dir', str(self.resources),
                '--output-dir', str(self.output), *extra_args]
            if cli:
                with patch.object(sys, 'argv', [str(DRIVER), *argv]), self.assertRaises(SystemExit) as exited:
                    runpy.run_path(str(DRIVER), run_name='__main__')
                result = exited.exception.code
            else:
                result = self.driver.main(argv)
        return result, stderr.getvalue(), stdout

    def test_reported_deps_library_and_test_are_copied_with_exact_binding(self):
        # A libtest and an rlib-only record with the same name are not production cdylibs.
        libtest = copy.deepcopy(self.library_record)
        libtest['profile']['test'] = True
        rlib = copy.deepcopy(self.library_record)
        rlib['target']['crate_types'] = ['rlib']
        rlib['filenames'] = [str(self.plugin.with_suffix('.rlib'))]
        diagnostic = {**self.library_record, 'reason': 'compiler-message'}
        result, error, raw_stdout = self.run_driver([[], diagnostic, libtest, rlib, *self.records], cli=True)
        self.assertEqual((result, error), (0, ''))
        self.assertTrue(self.output.is_symlink())
        for name, original in [('DandrumTrigger.clap', self.plugin), ('trigger-gui-test', self.executable)]:
            copied = self.output / name
            self.assertEqual(copied.read_bytes(), original.read_bytes())
            self.assertEqual(stat.S_IMODE(copied.stat().st_mode), stat.S_IMODE(original.stat().st_mode))
        self.assertNotEqual((self.output / 'DandrumTrigger.clap').read_bytes(), self.stale.read_bytes())
        self.assertEqual((self.output / 'DandrumTrigger.resources/nested/ui.bin').read_bytes(), b'UI resource\x00\xff')
        self.assertFalse((self.output / 'DandrumTrigger.resources').is_symlink())
        invocation = json.loads(self.invocation.read_text())
        command = [str(self.cargo), 'test', '--release', '--locked', '--jobs', '2',
                   '--manifest-path', str(self.manifest), '--features', 'trigger-editor',
                   '--test', 'trigger_gui', '--no-run', '--message-format=json']
        self.assertEqual(invocation, {'argv': command, 'cwd': str(self.manifest.parent),
                                      'target_dir': str(self.target), 'offline': 'true'})
        evidence = self.output / 'evidence'
        self.assertEqual((evidence / 'cargo.stdout.log').read_text(), raw_stdout)
        self.assertEqual((evidence / 'cargo.stderr.log').read_text(), 'retained cargo stderr\n')
        self.assertEqual(json.loads((evidence / 'invocation.json').read_text()),
                         {'command': command, 'cwd': str(self.manifest.parent), 'exit_status': 0})
        binding = json.loads((self.output / 'binding.json').read_text())
        self.assertEqual(binding['command'], command)
        for key, source, record, destination in [
            ('plugin', self.plugin, self.library_record, 'DandrumTrigger.clap'),
            ('test', self.executable, self.test_record, 'trigger-gui-test')]:
            artifact = binding['artifacts'][key]
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            self.assertEqual(artifact, {'source': str(source), 'copy': destination,
                                        'source_sha256': digest, 'copy_sha256': digest,
                                        'compiler_artifact': record})

    def test_explicit_parameter_target_selects_its_reported_executable(self):
        parameters = copy.deepcopy(self.test_record)
        executable = self.executable.with_name('trigger_parameters-0123456789abcdef')
        executable.write_bytes(b'actual parameter test executable')
        executable.chmod(0o751)
        parameters['target']['name'] = 'trigger_parameters'
        parameters['target']['src_path'] = str(self.manifest.parent / 'tests/trigger_parameters.rs')
        parameters['filenames'] = [str(executable)]
        parameters['executable'] = str(executable)
        result, error, _ = self.run_driver(
            [*self.records, parameters], cli=True,
            extra_args=['--test-target', 'trigger_parameters'])
        self.assertEqual((result, error), (0, ''))
        self.assertEqual((self.output / 'trigger-parameters-test').read_bytes(), executable.read_bytes())
        self.assertFalse((self.output / 'trigger-gui-test').exists())
        invocation = json.loads(self.invocation.read_text())
        self.assertEqual(invocation['argv'][invocation['argv'].index('--test') + 1], 'trigger_parameters')
        binding = json.loads((self.output / 'binding.json').read_text())
        self.assertEqual(binding['artifacts']['test']['source'], str(executable))
        self.assertEqual(binding['artifacts']['test']['compiler_artifact'], parameters)

    def test_missing_or_ambiguous_reported_artifacts_are_named_failures(self):
        no_so = copy.deepcopy(self.library_record)
        no_so['filenames'] = [str(self.plugin.with_suffix('.rlib'))]
        no_executable = copy.deepcopy(self.test_record)
        no_executable['executable'] = None
        cases = [
            ([self.test_record], 'production-cdylib'),
            ([no_so, self.test_record], 'production-cdylib'),
            ([self.library_record, self.library_record, self.test_record], 'production-cdylib'),
            ([self.library_record], 'trigger-gui-test'),
            ([self.library_record, no_executable], 'trigger-gui-test'),
            ([self.library_record, self.test_record, self.test_record], 'trigger-gui-test'),
        ]
        for records, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic, records=records):
                result, error, _ = self.run_driver(records)
                self.assertNotEqual(result, 0)
                self.assertIn(diagnostic, error)
                self.assertFalse(self.output.exists())

    def test_nonzero_cargo_retains_raw_evidence_and_refuses_plausible_artifacts(self):
        result, error, raw_stdout = self.run_driver(status=23)
        self.assertNotEqual(result, 0)
        self.assertIn('cargo-exit-status: 23', error)
        self.assertFalse(self.output.exists())
        stages = list(self.root.glob('.trigger-clap-native-fixture-*'))
        self.assertEqual(len(stages), 1)
        evidence = stages[0] / 'evidence'
        self.assertEqual((evidence / 'cargo.stdout.log').read_text(), raw_stdout)
        self.assertEqual((evidence / 'cargo.stderr.log').read_text(), 'retained cargo stderr\n')
        self.assertEqual(json.loads((evidence / 'invocation.json').read_text())['exit_status'], 23)
        self.assertFalse((stages[0] / 'binding.json').exists())

    def test_repeated_prepare_publishes_new_complete_fixture_and_preserves_previous_stage(self):
        self.assertEqual(self.run_driver()[0], 0)
        previous = self.output.resolve()
        previous_binding = (previous / 'binding.json').read_bytes()
        previous_plugin = (previous / 'DandrumTrigger.clap').read_bytes()
        self.plugin.write_bytes(b'next actual deps library')
        (self.resources / 'next.bin').write_bytes(b'next resource')
        self.assertEqual(self.run_driver()[0], 0)
        self.assertNotEqual(self.output.resolve(), previous)
        self.assertEqual((previous / 'binding.json').read_bytes(), previous_binding)
        self.assertEqual((previous / 'DandrumTrigger.clap').read_bytes(), previous_plugin)
        self.assertEqual((self.output / 'DandrumTrigger.clap').read_bytes(), b'next actual deps library')
        self.assertEqual((self.output / 'DandrumTrigger.resources/next.bin').read_bytes(), b'next resource')
        self.assertEqual(len(list(self.root.glob('.trigger-clap-native-fixture-*'))), 2)

    def test_failed_prepare_keeps_the_accepted_binding(self):
        self.assertEqual(self.run_driver()[0], 0)
        accepted = self.output.resolve()
        binding = (accepted / 'binding.json').read_bytes()
        for records, status in [([], 0), (self.records, 17)]:
            result, _, _ = self.run_driver(records, status)
            self.assertNotEqual(result, 0)
            self.assertEqual(self.output.resolve(), accepted)
            self.assertEqual((accepted / 'binding.json').read_bytes(), binding)
        # A reported path that is missing must not fall back to the stale convenience path.
        self.plugin.unlink()
        result, error, _ = self.run_driver()
        self.assertNotEqual(result, 0)
        self.assertIn(str(self.plugin), error)
        self.assertEqual(self.output.resolve(), accepted)
        self.plugin.write_bytes(b'restored actual library')
        self.resources.rename(self.resources.with_name('parked-resources'))
        result, error, _ = self.run_driver()
        self.assertNotEqual(result, 0)
        self.assertIn(str(self.resources), error)
        self.assertEqual(self.output.resolve(), accepted)
        self.assertEqual((accepted / 'binding.json').read_bytes(), binding)

    def test_existing_directory_file_or_unowned_alias_is_refused_without_cargo(self):
        directory = self.root / 'existing-directory'
        directory.mkdir()
        (directory / 'sentinel').write_bytes(b'preserve directory')
        regular = self.root / 'existing-file'
        regular.write_bytes(b'preserve regular file')
        foreign = self.root / '.trigger-clap-native-fixture-foreign'
        foreign.mkdir()
        (foreign / 'binding.json').write_text(json.dumps({'owner': 'another-driver', 'schema_version': 1}))
        alias = self.root / 'foreign-alias'
        alias.symlink_to(foreign.name)
        dangling = self.root / 'dangling-alias'
        dangling.symlink_to('missing-stage')
        malformed = self.root / '.trigger-clap-native-fixture-malformed'
        malformed.mkdir()
        (malformed / 'binding.json').write_text('not JSON')
        malformed_alias = self.root / 'malformed-alias'
        malformed_alias.symlink_to(malformed.name)
        for output in [directory, regular, alias, dangling, malformed_alias]:
            with self.subTest(output=output):
                self.output = output
                result, error, _ = self.run_driver()
                self.assertNotEqual(result, 0)
                self.assertIn('output-alias', error)
                self.assertFalse(self.invocation.exists())
        self.assertEqual((directory / 'sentinel').read_bytes(), b'preserve directory')
        self.assertEqual(regular.read_bytes(), b'preserve regular file')
        self.assertEqual(alias.resolve(), foreign)
        self.assertEqual(os.readlink(dangling), 'missing-stage')

    def test_copy_that_changes_the_bound_bytes_is_not_published(self):
        copy_file = self.driver.shutil.copy2

        def corrupted_copy(source, destination, **kwargs):
            result = copy_file(source, destination, **kwargs)
            if Path(destination).name == 'DandrumTrigger.clap':
                Path(destination).write_bytes(b'wrong copied plugin')
            return result

        with patch.object(self.driver.shutil, 'copy2', side_effect=corrupted_copy):
            result, error, _ = self.run_driver()
        self.assertNotEqual(result, 0)
        self.assertIn('artifact-copy-hash: plugin', error)
        self.assertFalse(self.output.exists())



if __name__ == '__main__':
    unittest.main()
