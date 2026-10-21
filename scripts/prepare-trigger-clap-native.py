#!/usr/bin/env python3
"""Prepare a native test fixture from Cargo's actual compiler-artifact records."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


OWNER = 'prepare-trigger-clap-native.py'
STAGE_PREFIX = '.trigger-clap-native-fixture-'


class FixtureError(Exception):
    """The requested native test fixture cannot be bound unambiguously."""


def select_artifacts(stdout, test_target):
    libraries = []
    tests = []
    for line in stdout.splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if not isinstance(record, dict) or record.get('reason') != 'compiler-artifact':
            continue
        target = record.get('target', {})
        if (target.get('name') == 'dandrum_clap_adapter'
                and 'cdylib' in target.get('crate_types', [])
                and record.get('profile', {}).get('test') is False):
            libraries.extend((filename, record) for filename in record.get('filenames', [])
                             if filename.endswith('.so'))
        if (target.get('name') == test_target and 'test' in target.get('kind', [])
                and record.get('executable')):
            tests.append((record['executable'], record))
    for rule, candidates in [('production-cdylib', libraries), (test_target.replace('_', '-') + '-test', tests)]:
        if len(candidates) != 1:
            raise FixtureError(f'{rule}: expected exactly one reported artifact; found {len(candidates)}')
    return {'plugin': libraries[0], 'test': tests[0]}


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def validate_output_alias(output):
    if not os.path.lexists(output):
        return
    if output.is_symlink():
        stage = output.resolve()
        if stage.parent == output.parent.resolve() and stage.name.startswith(STAGE_PREFIX):
            try:
                binding = json.loads((stage / 'binding.json').read_text())
            except (OSError, ValueError):
                pass
            else:
                if (isinstance(binding, dict) and binding.get('owner') == OWNER
                        and binding.get('schema_version') == 1):
                    return
    raise FixtureError(f'output-alias: refusing to replace an unowned path: {output}')


def prepare(args):
    output = args.output_dir.absolute()
    validate_output_alias(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=STAGE_PREFIX, dir=output.parent))
    evidence = stage / 'evidence'
    evidence.mkdir()
    manifest = args.manifest.absolute()
    command = [str(args.cargo), 'test', '--release', '--locked', '--jobs', '2',
               '--manifest-path', str(manifest), '--features', 'trigger-editor',
               '--test', args.test_target, '--no-run', '--message-format=json']
    invocation = {'command': command, 'cwd': str(manifest.parent), 'exit_status': None}
    invocation_path = evidence / 'invocation.json'
    invocation_path.write_text(json.dumps(invocation, indent=2) + '\n')
    environment = dict(os.environ, CARGO_TARGET_DIR=str(args.target_dir.absolute()))
    with (evidence / 'cargo.stdout.log').open('wb') as stdout, \
            (evidence / 'cargo.stderr.log').open('wb') as stderr:
        result = subprocess.run(command, cwd=manifest.parent, env=environment,
                                stdout=stdout, stderr=stderr)
    invocation['exit_status'] = result.returncode
    invocation_path.write_text(json.dumps(invocation, indent=2) + '\n')
    if result.returncode != 0:
        raise FixtureError(f'cargo-exit-status: {result.returncode}; evidence: {evidence}')
    artifacts = select_artifacts((evidence / 'cargo.stdout.log').read_text(errors='replace'), args.test_target)
    binding = {'schema_version': 1, 'owner': OWNER, **invocation, 'artifacts': {}}
    for name, destination in [('plugin', 'DandrumTrigger.clap'), ('test', args.test_target.replace('_', '-') + '-test')]:
        reported_path, record = artifacts[name]
        source = Path(reported_path)
        digest = sha256(source)
        copied = stage / destination
        shutil.copy2(source, copied)
        copied_digest = sha256(copied)
        if copied_digest != digest:
            raise FixtureError(f'artifact-copy-hash: {name}: copied bytes differ from {reported_path}')
        binding['artifacts'][name] = {
            'source': reported_path, 'copy': destination, 'source_sha256': digest,
            'copy_sha256': copied_digest, 'compiler_artifact': record,
        }
    shutil.copytree(args.resource_dir, stage / 'DandrumTrigger.resources')
    (stage / 'binding.json').write_text(json.dumps(binding, indent=2) + '\n')
    alias = stage.with_name(stage.name + '-alias')
    alias.symlink_to(stage.name, target_is_directory=True)
    os.replace(alias, output)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ['cargo', 'manifest', 'target-dir', 'resource-dir', 'output-dir']:
        parser.add_argument('--' + option, type=Path, required=True)
    parser.add_argument('--test-target', choices=['trigger_gui', 'trigger_parameters'], default='trigger_gui')
    args = parser.parse_args(argv)
    try:
        prepare(args)
    except (FixtureError, OSError) as error:
        print(f'{OWNER}: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
