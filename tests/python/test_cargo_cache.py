"""Fast cache ownership/entry-point contracts using a calibrated fake Cargo.

Real Cargo reuse, source edits and build boundaries belong to the slow lane.
"""
import json
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import time
import threading
import unittest
import uuid
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / 'scripts/cargo-cache.py'


class CargoCacheTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='cargo cache ')
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.repo = self.directory / 'checkout'
        self.repo.mkdir()
        subprocess.run(['git', 'init', '-q', str(self.repo)], check=True)
        self.cache = self.directory / 'shared cache'
        self.policy = self.repo / '.ai-rules.json'
        self.write_policy()
        self.calls = self.repo / 'calls.jsonl'
        self.cargo = self.directory / 'fake cargo'
        self.cargo.write_text(
            f'#!{sys.executable}\n'
            'import json,os,pathlib,sys,time\n'
            'if sys.argv[1:] == ["-vV"]:\n'
            ' print("cargo 1.96.0\\nfixture compiler identity");sys.exit(0)\n'
            'args=sys.argv[1:]\n'
            'manifest=pathlib.Path(args[args.index("--manifest-path")+1])\n'
            'build=pathlib.Path(os.environ["CARGO_BUILD_BUILD_DIR"])\n'
            'target=pathlib.Path(os.environ["CARGO_TARGET_DIR"])\n'
            'build.mkdir(parents=True,exist_ok=True);target.mkdir(parents=True,exist_ok=True)\n'
            'dependency=build/"dependency.bin"\n'
            'fresh=dependency.exists()\n'
            'if not fresh:dependency.write_bytes(b"compiled dependency")\n'
            'if os.environ.get("FAKE_CARGO_GROW"):\n'
            ' (build/"growth.bin").write_bytes(b"x"*int(os.environ["FAKE_CARGO_GROW"]))\n'
            'artifact=target/"program.txt"\n'
            'artifact.write_text((manifest.parent/"value.txt").read_text())\n'
            'row={"build":str(build),"target":str(target),"artifact":str(artifact),'
            '"fresh":fresh,"value":artifact.read_text(),"pid":os.getpid(),"makeflags":os.environ.get("MAKEFLAGS")}\n'
            f'with pathlib.Path({str(self.calls)!r}).open("a") as out:out.write(json.dumps(row)+"\\n")\n'
            'if os.environ.get("FAKE_CARGO_WAIT"):\n'
            ' release=pathlib.Path(os.environ["FAKE_CARGO_WAIT"])\n'
            ' release.with_suffix(".ready").write_text("ready")\n'
            ' while not release.exists():time.sleep(0.01)\n'
            'print(json.dumps(row))\n'
            'sys.exit(int(os.environ.get("FAKE_CARGO_EXIT","0")))\n')
        self.cargo.chmod(0o755)

    def write_policy(self, retained=64, growth=1):
        self.policy.write_text(json.dumps({'schemaVersion': 2, 'resources': {'disk': {
            'workloads': {'trigger-beta-verification': {
                'retainedBudgetMiB': retained, 'outputGrowthMiB': growth,
                'tempGrowthMiB': 1, 'reserveMiB': 1,
            }}}}}))

    def package(self, name, value):
        path = self.repo / 'src' / name
        path.mkdir(parents=True)
        (path / 'Cargo.toml').write_text(f'[package]\nname="{name}"\nversion="0.1.0"\n')
        (path / 'Cargo.lock').write_text('version = 4\n')
        (path / 'value.txt').write_text(value)
        return path / 'Cargo.toml'

    def command(self, *args):
        prefix = [sys.executable]
        if os.environ.get('DANDRUM_CACHE_TRACE_DIR'):
            path = Path(os.environ['DANDRUM_CACHE_TRACE_DIR'])
            path.mkdir(parents=True, exist_ok=True)
            count_file = str(path / (uuid.uuid4().hex + '.counts'))
            prefix += ['-c',
                'import runpy,sys,trace; sys.argv=sys.argv[1:]; '
                f't=trace.Trace(count=True,trace=False,infile={count_file!r},outfile={count_file!r}); '
                '\ntry:t.runfunc(runpy.run_path,sys.argv[0],run_name="__main__")'
                f'\nfinally:t.results().write_results(coverdir={str(path)!r})']
        return [*prefix, str(DRIVER), '--repo', str(self.repo),
                '--cache-root', str(self.cache), '--cargo', str(self.cargo), *args]

    def invoke(self, *args, env=None):
        return subprocess.run(self.command(*args),
            cwd=self.repo, env=dict(os.environ, RUSTC=str(self.cargo), **(env or {})),
            text=True, capture_output=True, timeout=10)

    def build(self, manifest, **kwargs):
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest), **kwargs)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_output_owners_share_warm_dependencies_and_source_edits_keep_reuse(self):
        first = self.package('first', 'first output')
        second = self.package('second', 'second output')
        a = self.build(first)
        b = self.build(second)
        self.assertEqual(a['build'], b['build'])
        self.assertNotEqual(a['target'], b['target'])
        self.assertFalse(a['fresh'])
        self.assertTrue(b['fresh'])
        self.assertEqual(Path(a['artifact']).read_text(), 'first output')
        self.assertEqual(Path(b['artifact']).read_text(), 'second output')
        (first.parent / 'value.txt').write_text('edited output')
        edited = self.build(first)
        self.assertTrue(edited['fresh'])
        self.assertEqual(edited['build'], a['build'])
        self.assertEqual(Path(edited['artifact']).read_text(), 'edited output')
        self.assertEqual(Path(b['artifact']).read_text(), 'second output')

    def test_cargo_path_options_cannot_bypass_output_accounting(self):
        manifest = self.package('app', 'ready')
        unknown = self.directory / 'unknown-output'
        unknown.mkdir()
        sentinel = unknown / 'keep'
        sentinel.write_text('unowned')
        for arguments, diagnostic in (
                (['--target-dir=' + str(unknown)], 'unknown private output'),
                (['--config', 'build.build-dir="elsewhere"'], 'cache-options'),
                (['--config=build.target-dir="elsewhere"'], 'cache-options'),
                (['--build-dir=elsewhere'], 'cache-options'),
                (['--target-dir'], 'cache-options')):
            result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest), *arguments)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn(diagnostic, result.stderr)
            self.assertFalse(self.calls.exists())
            self.assertEqual(sentinel.read_text(), 'unowned')

    def test_full_current_cache_refuses_growth_without_erasing_warm_state(self):
        manifest = self.package('app', 'ready')
        built = self.build(manifest)
        hot = Path(built['build']) / 'hot.bin'
        hot.write_bytes(b'x' * (3 * 1024 * 1024))
        before = self.calls.read_bytes()
        self.write_policy(retained=3)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('retention-budget', result.stderr)
        self.assertEqual(self.calls.read_bytes(), before, 'Refused producer must not run')
        self.assertEqual(hot.stat().st_size, 3 * 1024 * 1024)
        self.assertEqual(Path(built['artifact']).read_text(), 'ready')

    def test_cache_sibling_and_parent_spelled_outputs_are_counted(self):
        manifest = self.package('app', 'ready')
        for target in (self.cache / 'custom-products', self.cache / '..' / 'escaped-products'):
            result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest),
                                 '--target-dir', str(target))
            self.assertEqual(result.returncode, 0, result.stderr)
            actual = target.resolve()
            (actual / 'large.bin').write_bytes(b'x' * (3 * 1024 * 1024))
            self.write_policy(retained=3)
            result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest),
                                 '--target-dir', str(target))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('retention-budget', result.stderr)
            (actual / 'large.bin').unlink()
            self.write_policy()

    def test_leading_cargo_options_cannot_route_coverage_to_ordinary_storage(self):
        manifest = self.package('app', 'ready')
        result = self.invoke('cargo', '--', '--color', 'never', 'llvm-cov',
                             '--manifest-path', str(manifest))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cache-options', result.stderr)
        self.assertFalse(self.calls.exists())

    def test_closed_parent_jobserver_descriptors_are_not_advertised_to_cargo(self):
        manifest = self.package('app', 'ready')
        row = self.build(manifest, env={'MAKEFLAGS': '--jobserver-auth=999,1000 -j'})
        self.assertIsNone(row['makeflags'])
        self.assertEqual(Path(row['artifact']).read_text(), 'ready')

    def test_concurrent_first_build_waits_for_root_ownership_publication(self):
        manifest = self.package('app', 'ready')
        spec = importlib.util.spec_from_file_location('cargo_cache_concurrent', DRIVER)
        driver = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(driver)
        publishing, release = threading.Event(), threading.Event()
        replace = driver.os.replace
        def delayed(source, destination):
            if Path(destination) == self.cache / 'owner.json':
                publishing.set()
                if not release.wait(5):
                    raise RuntimeError('test failed to release initialization')
            return replace(source, destination)
        command = self.command('cargo', '--', 'build', '--manifest-path', str(manifest))
        result = []
        with mock.patch.dict(os.environ, {'RUSTC': str(self.cargo)}), mock.patch.object(driver.os, 'replace', side_effect=delayed):
            thread = threading.Thread(target=lambda: result.append(driver.main(command[command.index(str(DRIVER)) + 1:])))
            thread.start()
            self.assertTrue(publishing.wait(5))
            second = subprocess.Popen(command, cwd=self.repo, env=dict(os.environ), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                time.sleep(0.15)
                self.assertIsNone(second.poll(), 'second builder must wait for the initializing owner')
            finally:
                release.set()
                thread.join(5)
                stdout, stderr = second.communicate(timeout=5)
        self.assertEqual(result, [0])
        self.assertEqual(second.returncode, 0, stderr)
        self.assertTrue(json.loads(stdout)['fresh'])

    def test_unmanaged_cargo_operations_cannot_destroy_or_bypass_the_shared_cache(self):
        manifest = self.package('app', 'ready')
        row = self.build(manifest)
        calls = self.calls.read_bytes()
        for operation in ('clean', 'mutants', 'install'):
            result = self.invoke('cargo', '--', operation, '--manifest-path', str(manifest))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('cache-options', result.stderr)
            self.assertEqual(self.calls.read_bytes(), calls)
            self.assertTrue(Path(row['build']).is_dir())

    def test_default_location_follows_common_git_or_explicit_override_and_standalone_fallback(self):
        manifest = self.package('app', 'ready')
        command = self.command('cargo', '--', 'build', '--manifest-path', str(manifest))
        index = command.index('--cache-root')
        del command[index:index + 2]
        environment = dict(os.environ, RUSTC=str(self.cargo))
        environment.pop('DANDRUM_CARGO_CACHE_ROOT', None)
        for expected, override, hide_git in (
                (self.repo / '.git/dandrum-cargo-cache', None, False),
                (self.directory / 'explicit-cache', self.directory / 'explicit-cache', False),
                (self.repo / 'build/cargo-cache', None, True)):
            if hide_git:
                (self.repo / '.git').rename(self.repo / 'parked-git')
            env = dict(environment)
            if override:
                env['DANDRUM_CARGO_CACHE_ROOT'] = str(override)
            result = subprocess.run(command, cwd=self.repo, env=env, text=True, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(Path(json.loads(result.stdout)['build']).parents[2], expected)

    def test_dependency_generation_change_retires_old_owned_cache_and_preserves_evidence(self):
        manifest = self.package('app', 'ready')
        old = self.build(manifest)
        (Path(old['build']) / 'old.bin').write_bytes(b'x' * (3 * 1024 * 1024))
        self.write_policy(retained=4)
        evidence = self.cache / 'retained-evidence.txt'
        evidence.write_text('required proof')
        (manifest.parent / 'Cargo.lock').write_text('version = 4\n# changed resolved inputs\n')
        new = self.build(manifest)
        self.assertNotEqual(old['build'], new['build'])
        self.assertFalse(Path(old['build']).exists())
        self.assertEqual(evidence.read_text(), 'required proof')
        self.assertTrue(Path(new['artifact']).exists())

    def test_unknown_cache_directory_is_preserved_and_producer_refused(self):
        manifest = self.package('app', 'ready')
        self.cache.mkdir()
        unknown = self.cache / 'important.txt'
        unknown.write_text('not owned by the driver')
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cache-ownership', result.stderr)
        self.assertEqual(unknown.read_text(), 'not owned by the driver')
        self.assertFalse(self.calls.exists())

    def start_waiting_build(self, manifest):
        release = self.directory / 'release'
        process = subprocess.Popen(
            self.command('cargo', '--', 'build', '--manifest-path', str(manifest)),
            cwd=self.repo, env=dict(os.environ, RUSTC=str(self.cargo), FAKE_CARGO_WAIT=str(release)),
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

        def finish():
            release.write_text('finish')
            process.communicate(timeout=5)

        self.addCleanup(finish)
        deadline = time.monotonic() + 5
        while not release.with_suffix('.ready').exists():
            self.assertIsNone(process.poll(), 'Producer exited before reaching its barrier')
            self.assertLess(time.monotonic(), deadline, 'Producer did not reach its barrier')
            time.sleep(0.01)
        return process, release

    def test_active_producer_prevents_generation_retirement(self):
        manifest = self.package('app', 'ready')
        process, release = self.start_waiting_build(manifest)
        old = json.loads(self.calls.read_text().splitlines()[0])
        (manifest.parent / 'Cargo.lock').write_text('version = 4\n# new generation\n')
        result = self.invoke('--lock-timeout', '0.1', 'cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cache-busy', result.stderr)
        self.assertTrue(Path(old['build']).is_dir())
        self.assertEqual(len(self.calls.read_text().splitlines()), 1)
        release.write_text('finish')
        process.communicate(timeout=5)
        self.assertEqual(process.returncode, 0)

    def test_child_keeps_lease_after_controller_loss_and_uncertain_run_is_preserved(self):
        manifest = self.package('app', 'ready')
        process, release = self.start_waiting_build(manifest)
        old = json.loads(self.calls.read_text().splitlines()[0])
        process.kill()
        process.wait(timeout=5)
        result = self.invoke('--lock-timeout', '0', 'cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertIn('cache-busy', result.stderr)
        self.assertTrue(Path(old['build']).is_dir())
        release.write_text('finish')
        process.communicate(timeout=5)
        result = self.invoke('--lock-timeout', '0', 'cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertIn('cache-active', result.stderr)
        self.assertTrue(Path(old['artifact']).is_file())

    def test_failed_producer_preserves_cache_and_propagates_exit(self):
        manifest = self.package('app', 'ready')
        built = self.build(manifest)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest),
                             env={'FAKE_CARGO_EXIT': '23'})
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertTrue(Path(built['build']).is_dir())
        self.assertTrue(self.build(manifest)['fresh'])

    def test_signalled_producer_requires_inspection_before_reuse(self):
        manifest = self.package('app', 'ready')
        process, release = self.start_waiting_build(manifest)
        row = json.loads(self.calls.read_text().splitlines()[0])
        os.kill(row['pid'], 9)
        process.communicate(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertIn('cache-active', result.stderr)
        self.assertTrue(Path(row['build']).is_dir())

    def test_changed_ownership_metadata_refuses_production(self):
        manifest = self.package('app', 'ready')
        row = self.build(manifest)
        data = Path(row['build'])
        for marker in (self.cache / 'owner.json', data.parent / 'owner.json', data / '.dandrum-cache-owner.json'):
            original = marker.read_bytes()
            before = self.calls.read_bytes()
            marker.write_text('{}')
            result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('cache-ownership', result.stderr)
            self.assertEqual(self.calls.read_bytes(), before)
            marker.write_bytes(original)

    def test_private_output_replacement_is_refused_and_removed_output_can_be_recreated(self):
        manifest = self.package('app', 'ready')
        target = self.directory / 'outputs'
        env = {'CARGO_TARGET_DIR': str(target)}
        self.build(manifest, env=env)
        marker = target / '.dandrum-output-owner.json'
        original = marker.read_bytes()
        marker.write_text('{}')
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest), env=env)
        self.assertIn('private output was replaced', result.stderr)
        marker.write_bytes(original)
        shutil.rmtree(target)
        self.assertTrue(self.build(manifest, env=env)['fresh'])

    def test_adopted_obsolete_generation_is_retired_only_under_capacity_pressure(self):
        manifest = self.package('app', 'ready')
        target = manifest.parent / 'target'
        target.mkdir()
        coverage = target / 'llvm-cov-target'
        coverage.mkdir()
        (target / 'warm').write_bytes(b'x' * (3 * 1024 * 1024))
        self.assertEqual(self.invoke('--ordinary', str(target), 'adopt').returncode, 0)
        self.build(manifest)
        self.write_policy(retained=4)
        (manifest.parent / 'Cargo.lock').write_text('# changed dependency graph')
        self.build(manifest)
        self.assertFalse(target.exists())

    def test_adoption_refuses_missing_tracked_or_already_populated_paths(self):
        manifest = self.package('app', 'ready')
        target = manifest.parent / 'target'
        target.mkdir()
        (target / 'warm').write_text('keep')
        result = self.invoke('adopt')
        self.assertIn('explicit ordinary target', result.stderr)
        result = self.invoke('--ordinary', str(target / 'missing'), 'adopt')
        self.assertIn('could not be qualified', result.stderr)
        subprocess.run(['git', '-C', str(self.repo), 'add', str(target / 'warm')], check=True)
        result = self.invoke('--ordinary', str(target), 'adopt')
        self.assertIn('tracked files are preserved', result.stderr)
        subprocess.run(['git', '-C', str(self.repo), 'rm', '--cached', str(target / 'warm')], check=True, capture_output=True)
        self.build(manifest)
        result = self.invoke('--ordinary', str(target), 'adopt')
        self.assertIn('existing managed data', result.stderr)
        self.assertEqual((target / 'warm').read_text(), 'keep')

    def test_coverage_uses_separate_storage_and_preserves_ordinary_reuse(self):
        manifest = self.package('app', 'ready')
        normal = self.build(manifest)
        result = self.invoke('cargo', '--', 'llvm-cov', '--manifest-path', str(manifest))
        self.assertEqual(result.returncode, 0, result.stderr)
        coverage = json.loads(result.stdout)
        self.assertNotEqual(coverage['build'], normal['build'])
        self.assertNotEqual(coverage['target'], normal['target'])
        self.assertFalse(coverage['fresh'])
        self.assertTrue(self.build(manifest)['fresh'])

    def test_explicit_adoption_reuses_existing_cache_at_original_path(self):
        manifest = self.package('app', 'ready')
        existing = self.repo / 'src/app/target'
        existing.mkdir()
        (existing / 'dependency.bin').write_bytes(b'warm existing dependency')
        result = self.invoke('--ordinary', str(existing), 'adopt')
        self.assertEqual(result.returncode, 0, result.stderr)
        built = self.build(manifest)
        self.assertEqual(Path(built['build']), existing)
        self.assertTrue(built['fresh'])
        self.assertEqual((existing / 'dependency.bin').read_bytes(), b'warm existing dependency')

    def test_replaced_cache_data_path_is_preserved(self):
        manifest = self.package('app', 'ready')
        built = self.build(manifest)
        original = Path(built['build'])
        original.rename(original.with_name('original'))
        outside = self.directory / 'outside'
        outside.mkdir()
        (outside / 'proof').write_text('preserve')
        original.symlink_to(outside, target_is_directory=True)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cache-ownership', result.stderr)
        self.assertEqual((outside / 'proof').read_text(), 'preserve')

    def test_inactive_generation_stays_warm_while_there_is_room(self):
        manifest = self.package('app', 'ready')
        original_lock = (manifest.parent / 'Cargo.lock').read_bytes()
        first = self.build(manifest)
        (manifest.parent / 'Cargo.lock').write_text('version = 4\n# another graph\n')
        second = self.build(manifest)
        self.assertTrue(Path(first['build']).is_dir())
        (manifest.parent / 'Cargo.lock').write_bytes(original_lock)
        reused = self.build(manifest)
        self.assertEqual(reused['build'], first['build'])
        self.assertTrue(reused['fresh'])
        self.assertTrue(Path(second['build']).is_dir())

    def test_unexpected_growth_is_reported_and_blocks_the_next_producer(self):
        manifest = self.package('app', 'ready')
        self.write_policy(retained=3)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest),
                             env={'FAKE_CARGO_GROW': str(4 * 1024 * 1024)})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('retention-overrun', result.stderr)
        built = json.loads(result.stdout)
        self.assertTrue(Path(built['artifact']).exists())
        before = self.calls.read_bytes()
        again = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertIn('retention-budget', again.stderr)
        self.assertEqual(self.calls.read_bytes(), before)

    def test_external_output_is_accounted_and_unknown_outputs_are_preserved(self):
        manifest = self.package('app', 'ready')
        target = self.directory / 'private output'
        first = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest), '--target-dir', str(target))
        self.assertEqual(first.returncode, 0, first.stderr)
        (target / 'large.bin').write_bytes(b'x' * (3 * 1024 * 1024))
        self.write_policy(retained=3)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest), '--target-dir', str(target))
        self.assertIn('retention-budget', result.stderr)
        self.assertTrue((target / 'large.bin').exists())
        unknown = self.directory / 'unknown output'
        unknown.mkdir()
        (unknown / 'proof').write_text('preserve')
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest), '--target-dir', str(unknown))
        self.assertIn('cache-ownership', result.stderr)
        self.assertEqual((unknown / 'proof').read_text(), 'preserve')

    def test_adoption_requires_successful_git_inspection(self):
        self.package('app', 'ready')
        target = self.repo / 'src/app/target'
        target.mkdir()
        (target / 'warm').write_text('keep')
        (self.repo / '.git').rename(self.repo / 'hidden-git')
        result = self.invoke('--ordinary', str(target), 'adopt')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cache-adopt', result.stderr)
        self.assertFalse((target / '.dandrum-cache-owner.json').exists())
        self.assertEqual((target / 'warm').read_text(), 'keep')

    def test_adoption_preserves_git_metadata_even_when_named_target(self):
        self.package('app', 'ready')
        target = self.repo / '.git/worktrees/target'
        target.mkdir(parents=True)
        (target / 'HEAD').write_text('metadata sentinel')
        result = self.invoke('--ordinary', str(target), 'adopt')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('cache-adopt', result.stderr)
        self.assertFalse((target / '.dandrum-cache-owner.json').exists())
        self.assertEqual((target / 'HEAD').read_text(), 'metadata sentinel')

    def test_low_cache_temporary_or_private_output_space_prevents_production(self):
        manifest = self.package('app', 'ready')
        spec = importlib.util.spec_from_file_location('cargo_cache_test', DRIVER)
        driver = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(driver)
        temporary = self.directory / 'other temporary volume'
        temporary.mkdir()
        output = self.directory / 'private outputs'
        command = self.command('cargo', '--', 'build', '--manifest-path', str(manifest), '--target-dir', str(output))
        for deficient, diagnostic in ((temporary, 'temporary-reserve'), (self.cache, 'disk-reserve'),
                                       (output, 'output-reserve')):
            error = io.StringIO()
            def space(path):
                return type('Space', (), {'free': 0 if Path(path) == deficient else 1024**4})()
            with mock.patch.dict(os.environ, {'RUSTC': str(self.cargo)}), \
                    mock.patch.object(driver.tempfile, 'gettempdir', return_value=str(temporary)), \
                    mock.patch.object(driver.shutil, 'disk_usage', side_effect=space), \
                    contextlib.redirect_stderr(error):
                result = driver.main(command[command.index(str(DRIVER)) + 1:])
            self.assertNotEqual(result, 0)
            self.assertIn(diagnostic, error.getvalue())
            self.assertFalse(self.calls.exists())
        self.write_policy(retained=-1)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertIn('cache-policy', result.stderr)
        self.assertFalse(self.calls.exists())

    def test_complete_publication_command_shares_the_cache_lease(self):
        manifest = self.package('app', 'ready')
        result = self.invoke('run', '--', str(self.cargo), 'build', '--manifest-path', str(manifest))
        self.assertEqual(result.returncode, 0, result.stderr)
        published = json.loads(result.stdout)
        self.assertTrue(Path(published['artifact']).exists())
        self.assertTrue(self.build(manifest)['fresh'])

    def test_interrupted_helper_preserves_the_outer_cache_marker(self):
        manifest = self.package('app', 'ready')
        result = self.invoke('run', '--', str(self.cargo), 'build', '--manifest-path', str(manifest),
                             env={'FAKE_CARGO_EXIT': '125'})
        self.assertEqual(result.returncode, 125)
        result = self.invoke('cargo', '--', 'build', '--manifest-path', str(manifest))
        self.assertIn('cache-active', result.stderr)


if __name__ == '__main__':
    unittest.main()
