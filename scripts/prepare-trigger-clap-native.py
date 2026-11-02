#!/usr/bin/env python3
"""Bind a prebuilt product and Cargo-reported test into a private native fixture."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid


OWNER = 'prepare-trigger-clap-native.py'
STAGE_PREFIX = '.trigger-clap-native-fixture-'


class FixtureError(Exception):
    """The requested native test fixture cannot be bound unambiguously."""


class FixtureInterrupted(FixtureError):
    """A signalled compiler may have surviving children; preserve its markers."""


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
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(str(output) + '.lease', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise FixtureError('fixture-busy: active reader or publisher is preserved')
        active = Path(str(output) + '.active.json')
        if active.exists() or active.is_symlink():
            raise FixtureError(f'fixture-interrupted: unfinished preparation is preserved: {active}')
        validate_output_alias(output)
        previous = output.resolve() if output.is_symlink() else None
        if previous is not None and os.path.lexists(previous / '.owner.json'):
            require_owned_stage(previous, output)
        failure = Path(str(output) + '.last-failure')
        if failure.exists() or failure.is_symlink():
            require_owned_stage(failure, output)
        stage = output.parent / (STAGE_PREFIX + uuid.uuid4().hex)
        with active.open('x') as record:
            json.dump({'owner': OWNER, 'pid': os.getpid(), 'stage': str(stage)}, record)
        try:
            publish(args, output, previous, failure, descriptor, stage)
        except FixtureInterrupted:
            raise
        except (FixtureError, OSError):
            if not stage.exists():
                active.unlink()
            raise
        active.unlink()
    finally:
        os.close(descriptor)


def require_owned_stage(stage, output):
    marker = stage / '.owner.json'
    expected = {'owner': OWNER, 'output': str(output),
                'identity': [stage.lstat().st_dev, stage.lstat().st_ino]}
    if stage.is_symlink() or marker.is_symlink() or not marker.is_file() or json.loads(marker.read_text()) != expected:
        raise FixtureError(f'fixture-ownership: unknown stage is preserved: {stage}')


def publish(args, output, previous, failure, descriptor, stage):
    stage.mkdir()
    (stage / '.owner.json').write_text(json.dumps({'owner': OWNER, 'output': str(output),
        'identity': [stage.stat().st_dev, stage.stat().st_ino]}))
    try:
        build_stage(args, output, stage, previous, descriptor)
    except FixtureInterrupted:
        raise
    except (FixtureError, OSError) as error:
        # Retain the latest failed attempt's raw logs, not another full binary
        # copy. The accepted alias and its exact binding stay untouched.
        for path in stage.iterdir():
            if path.name not in ('.owner.json', 'evidence'):
                if path.is_dir() and not path.is_symlink():
                    shutil.rmtree(path)
                else:
                    path.unlink()
        (stage / 'failure.txt').write_text(str(error) + '\n')
        if failure.exists():
            require_owned_stage(failure, output)
            shutil.rmtree(failure)
        stage.rename(failure)
        raise FixtureError(f'{error}; latest failure evidence: {failure}') from error
    if previous is not None and (previous / '.owner.json').exists():
        require_owned_stage(previous, output)
        shutil.rmtree(previous)


def build_stage(args, output, stage, previous, descriptor):
    evidence = stage / 'evidence'
    evidence.mkdir()
    if previous is not None:
        (evidence / 'previous-binding.json').write_bytes((previous / 'binding.json').read_bytes())
    manifest = args.manifest.absolute()
    command = [str(args.cargo), 'test', '--release', '--locked', '--jobs', '2',
               '--manifest-path', str(manifest), '--features', 'trigger-editor',
               '--test', args.test_target, '--no-run', '--message-format=json']
    invocation = {'command': command, 'cwd': str(manifest.parent), 'exit_status': None}
    invocation_path = evidence / 'invocation.json'
    invocation_path.write_text(json.dumps(invocation, indent=2) + '\n')
    environment = dict(os.environ, CARGO_TARGET_DIR=str(args.target_dir.absolute()))
    leases = (descriptor,)
    if environment.get('DANDRUM_CARGO_CACHE_LEASE_FD'):
        leases += (int(environment['DANDRUM_CARGO_CACHE_LEASE_FD']),)
    with (evidence / 'cargo.stdout.log').open('wb') as stdout, \
            (evidence / 'cargo.stderr.log').open('wb') as stderr:
        result = subprocess.run(command, cwd=manifest.parent, env=environment,
                                stdout=stdout, stderr=stderr, pass_fds=leases)
    invocation['exit_status'] = result.returncode
    invocation_path.write_text(json.dumps(invocation, indent=2) + '\n')
    if result.returncode < 0:
        raise FixtureInterrupted(f'fixture-interrupted: Cargo signal {-result.returncode}; evidence: {evidence}')
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
    try:
        alias.symlink_to(stage.name, target_is_directory=True)
        os.replace(alias, output)
    finally:
        alias.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ['cargo', 'manifest', 'target-dir', 'resource-dir', 'output-dir', 'product']:
        parser.add_argument('--' + option, type=Path, required=True)
    parser.add_argument('--test-target', choices=['trigger_gui', 'trigger_parameters'], default='trigger_gui')
    args = parser.parse_args(argv)
    try:
        prepare(args)
    except FixtureInterrupted as error:
        print(f'{OWNER}: {error}', file=sys.stderr)
        return 125  # Shared-cache run protocol: preserve the outer active marker.
    except (FixtureError, OSError, ValueError) as error:
        print(f'{OWNER}: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
