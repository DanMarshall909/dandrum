#!/usr/bin/env python3
"""Bind a prebuilt product and Cargo-reported test into a private native fixture."""
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


def select_test(stdout, test_target):
    tests = []
    for line in stdout.splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if not isinstance(record, dict) or record.get('reason') != 'compiler-artifact':
            continue
        target = record.get('target', {})
        if (target.get('name') == test_target and 'test' in target.get('kind', [])
                and record.get('executable')):
            tests.append((record['executable'], record))
    if len(tests) != 1:
        raise FixtureError(f'{test_target.replace("_", "-")}-test: '
                           f'expected exactly one reported artifact; found {len(tests)}')
    return tests[0]


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
    executable, record = select_test(
        (evidence / 'cargo.stdout.log').read_text(errors='replace'), args.test_target)
    artifacts = {'plugin': (str(args.product.absolute()), None), 'test': (executable, record)}
    binding = {'schema_version': 1, 'owner': OWNER, **invocation, 'artifacts': {}}
    for name, destination in [('plugin', 'DandrumTrigger.clap'), ('test', args.test_target.replace('_', '-') + '-test')]:
        reported_path, record = artifacts[name]
        source = Path(reported_path)
        if not source.is_file():
            raise FixtureError(f'artifact-source: {name}: not a regular file: {source}')
        digest = sha256(source)
        copied = stage / destination
        shutil.copy2(source, copied)
        copied_digest = sha256(copied)
        if copied_digest != digest:
            raise FixtureError(f'artifact-copy-hash: {name}: copied bytes differ from {reported_path}')
        binding['artifacts'][name] = {
            'source': reported_path, 'copy': destination, 'source_sha256': digest,
            'copy_sha256': copied_digest,
        }
        if name == 'plugin':
            binding['artifacts'][name]['provenance'] = 'prebuilt-product'
        else:
            binding['artifacts'][name]['compiler_artifact'] = record
    resource_source = args.resource_dir.absolute()
    resource_copy = stage / 'DandrumTrigger.resources'
    resource_hashes = {path.relative_to(resource_source).as_posix(): sha256(path)
                       for path in sorted(resource_source.rglob('*')) if path.is_file()}
    shutil.copytree(resource_source, resource_copy)
    copied_hashes = {path.relative_to(resource_copy).as_posix(): sha256(path)
                     for path in sorted(resource_copy.rglob('*')) if path.is_file()}
    if copied_hashes != resource_hashes:
        raise FixtureError('resource-copy-hash: copied resource files differ from input tree')
    binding['resources'] = {
        'source': str(resource_source), 'copy': resource_copy.name,
        'files': [{'path': relative, 'source_sha256': digest, 'copy_sha256': copied_hashes[relative]}
                  for relative, digest in sorted(resource_hashes.items())],
    }
    (stage / 'binding.json').write_text(json.dumps(binding, indent=2) + '\n')
    alias = stage.with_name(stage.name + '-alias')
    alias.symlink_to(stage.name, target_is_directory=True)
    os.replace(alias, output)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ['cargo', 'manifest', 'target-dir', 'resource-dir', 'output-dir', 'product']:
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
