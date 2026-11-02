#!/usr/bin/env python3
"""Run maintained Rust commands with shared intermediates and private outputs."""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import time


OWNER = 'dandrum-cargo-cache-v1'
MIB = 1024 * 1024


class CacheError(Exception):
    """Managed cache work cannot proceed without risking retained state."""


def write_json(path, value):
    with tempfile.NamedTemporaryFile(mode='w', dir=path.parent, delete=False) as output:
        temporary = Path(output.name)
        try:
            json.dump(value, output, indent=2)
            output.write('\n')
            output.close()
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)


def identity(path):
    info = path.lstat()
    return [info.st_dev, info.st_ino]


def require_plain_path(path):
    if any(parent.is_symlink() for parent in (path, *path.parents)):
        raise CacheError(f'cache-ownership: linked path is preserved: {path}')


def initialize_cache(cache):
    require_plain_path(cache)
    if not cache.exists():
        cache.mkdir(parents=True)
    marker = cache / 'owner.json'
    if not marker.exists():
        if any(cache.iterdir()):
            raise CacheError(f'cache-ownership: unknown directory is preserved: {cache}')
        write_json(marker, {'owner': OWNER, 'path': str(cache), 'identity': identity(cache)})
    expected = {'owner': OWNER, 'path': str(cache), 'identity': identity(cache)}
    if marker.is_symlink() or json.loads(marker.read_text()) != expected:
        raise CacheError(f'cache-ownership: changed root is preserved: {cache}')
    (cache / 'generations').mkdir(exist_ok=True)


def select_generation(cache, key):
    root = cache / 'generations'
    require_plain_path(root)
    previous_generations = list(root.iterdir())
    for previous in previous_generations:
        require_plain_path(previous)
        marker = previous / 'owner.json'
        expected = {'owner': OWNER, 'key': previous.name, 'identity': identity(previous)}
        record = json.loads(marker.read_text()) if marker.is_file() and not marker.is_symlink() else {}
        if any(record.get(name) != value for name, value in expected.items()):
            raise CacheError(f'cache-ownership: unknown generation is preserved: {previous}')
        generation_data(previous)
        if (previous / 'active.json').exists():
            raise CacheError(f'cache-active: unfinished producer is preserved: {previous}')
    generation = root / key
    if not generation.exists():
        generation.mkdir()
        paths = {}
        for lane in ('ordinary', 'coverage'):
            path = generation / lane
            path.mkdir()
            mark_data(path, generation, lane)
            paths[lane] = str(path)
        write_json(generation / 'owner.json', {'owner': OWNER, 'key': key,
                   'identity': identity(generation), 'paths': paths})
    return generation


def retire_generation(generation):
    for path in sorted(set(generation_data(generation).values()), key=lambda p: len(p.parts)):
        if path.exists() and generation not in path.parents:
            shutil.rmtree(path)
    shutil.rmtree(generation)


def mark_data(path, generation, lane):
    write_json(path / '.dandrum-cache-owner.json',
               {'owner': OWNER, 'generation': str(generation), 'lane': lane, 'identity': identity(path)})


def generation_data(generation):
    record = json.loads((generation / 'owner.json').read_text())
    result = {}
    for lane in ('ordinary', 'coverage'):
        path = Path(record['paths'][lane])
        require_plain_path(path)
        marker = path / '.dandrum-cache-owner.json'
        expected = {'owner': OWNER, 'generation': str(generation), 'lane': lane, 'identity': identity(path)}
        if marker.is_symlink() or json.loads(marker.read_text()) != expected:
            raise CacheError(f'cache-ownership: replaced data path is preserved: {path}')
        result[lane] = path
    return result


def adopt(generation, repo, ordinary, coverage):
    if ordinary is None:
        raise CacheError('cache-adopt: an explicit ordinary target is required')
    paths = generation_data(generation)
    proposed = {'ordinary': ordinary.absolute(),
                'coverage': coverage.absolute() if coverage else ordinary.absolute() / 'llvm-cov-target'}
    if not proposed['coverage'].exists():
        proposed['coverage'] = paths['coverage']
    for lane, path in proposed.items():
        if path == paths[lane]:
            continue
        require_plain_path(path)
        if (not path.is_dir() or repo not in path.parents or path.name not in ('target', 'rust-target', 'llvm-cov-target')
                or '.git' in path.relative_to(repo).parts
                or (path / '.git').exists() or (path / 'Cargo.toml').exists()
                or path.stat().st_dev != generation.stat().st_dev
                or (path / '.dandrum-cache-owner.json').exists()):
            raise CacheError(f'cache-adopt: explicit generated target could not be qualified: {path}')
        tracked = subprocess.run(['git', '-C', str(repo), 'ls-files', '--', str(path)],
                                 capture_output=True, text=True)
        if tracked.returncode != 0:
            raise CacheError(f'cache-adopt: Git ownership inspection failed: {path}')
        if tracked.stdout:
            raise CacheError(f'cache-adopt: tracked files are preserved: {path}')
        if set(paths[lane].iterdir()) != {paths[lane] / '.dandrum-cache-owner.json'}:
            raise CacheError('cache-adopt: existing managed data must be preserved')
    for lane, path in proposed.items():
        if path != paths[lane]:
            mark_data(path, generation, lane)
            shutil.rmtree(paths[lane])
    record = json.loads((generation / 'owner.json').read_text())
    record['paths'] = {lane: str(path) for lane, path in proposed.items()}
    write_json(generation / 'owner.json', record)


@contextmanager
def cache_lease(cache, timeout, name='lease'):
    descriptor = os.open(cache / name, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        deadline = time.monotonic() + timeout
        while True:
            try:
                fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise CacheError('cache-busy: another producer or its child retains the lease')
                time.sleep(0.05)
        yield descriptor
    finally:
        # Do not explicitly unlock: an inherited child descriptor must retain
        # protection if this controller exits before that child.
        os.close(descriptor)


def measured_bytes(roots):
    seen = set()
    total = 0
    pending = list(roots)
    entries = 0
    while pending:
        path = pending.pop()
        info = path.lstat()
        key = (info.st_dev, info.st_ino)
        if key in seen:
            continue
        seen.add(key)
        total += info.st_blocks * 512
        entries += 1
        if entries > 200000:
            raise CacheError('cache-inventory: entry limit reached; no producer started')
        if stat.S_ISDIR(info.st_mode):
            pending.extend(path.iterdir())
    return total


def retained_bytes(cache):
    roots = [cache / 'generations']
    for generation in (cache / 'generations').iterdir():
        roots.extend(generation_data(generation).values())
    roots.extend(output_paths(cache))
    return measured_bytes(roots)


def output_paths(cache):
    registry = cache / 'outputs.json'
    records = json.loads(registry.read_text()) if registry.exists() else []
    result = []
    for record in records:
        path = Path(record['path'])
        if not path.exists() and not path.is_symlink():
            continue  # The output owner has already removed its private tree.
        require_plain_path(path)
        marker = path / '.dandrum-output-owner.json'
        expected = {'owner': OWNER, 'cache': str(cache), 'identity': identity(path)}
        if marker.is_symlink() or not marker.exists() or json.loads(marker.read_text()) != expected or record['identity'] != identity(path):
            raise CacheError(f'cache-ownership: private output was replaced: {path}')
        result.append(path)
    return result


def register_output(cache, path):
    require_plain_path(path)
    if cache / 'generations' in path.parents:
        return
    existing = output_paths(cache)
    if path in existing:
        return
    if path.exists() and any(path.iterdir()):
        raise CacheError(f'cache-ownership: unknown private output is preserved: {path}')
    path.mkdir(parents=True, exist_ok=True)
    write_json(path / '.dandrum-output-owner.json',
               {'owner': OWNER, 'cache': str(cache), 'identity': identity(path)})
    write_json(cache / 'outputs.json', [{'path': str(p), 'identity': identity(p)} for p in [*existing, path]])


def admit(repo, cache, current):
    policy = json.loads((repo / '.ai-rules.json').read_text())['resources']['disk']['workloads']['trigger-beta-verification']
    for name in ('outputGrowthMiB', 'retainedBudgetMiB', 'tempGrowthMiB', 'reserveMiB'):
        if type(policy[name]) is not int or policy[name] < 0:
            raise CacheError(f'cache-policy: invalid {name}')
    retained = retained_bytes(cache)
    growth = policy['outputGrowthMiB'] * MIB
    budget = policy['retainedBudgetMiB'] * MIB
    needed = (policy['outputGrowthMiB'] + policy['tempGrowthMiB'] + policy['reserveMiB']) * MIB
    previous = sorted((p for p in (cache / 'generations').iterdir() if p != current),
                      key=lambda p: json.loads((p / 'owner.json').read_text()).get('last_used_ns', 0))
    for generation in previous:
        if retained + growth <= budget and shutil.disk_usage(cache).free >= needed:
            break
        retire_generation(generation)
        retained = retained_bytes(cache)
    if retained + growth > budget:
        raise CacheError(f'retention-budget: retained={retained}, growth={growth}, '
                         f'budget={budget}; current warm cache preserved')
    free = shutil.disk_usage(cache).free
    if free < needed:
        raise CacheError(f'disk-reserve: free={free}, required={needed}; current warm cache preserved')
    temporary = Path(tempfile.gettempdir())
    temporary_needed = (policy['tempGrowthMiB'] + policy['reserveMiB']) * MIB
    if shutil.disk_usage(temporary).free < temporary_needed:
        raise CacheError(f'temporary-reserve: required={temporary_needed}, path={temporary}')
    for output in output_paths(cache):
        if shutil.disk_usage(output).free < (policy['outputGrowthMiB'] + policy['reserveMiB']) * MIB:
            raise CacheError(f'output-reserve: private output volume cannot fit growth: {output}')
    return budget, retained


def generation_key(repo, cargo):
    digest = hashlib.sha256()
    for tool in (cargo, os.environ.get('RUSTC', str(Path.home() / '.cargo/bin/rustc'))):
        digest.update(subprocess.check_output([str(tool), '-vV']))
    for path in sorted((repo / 'src').glob('*/Cargo.*')):
        if path.name in ('Cargo.toml', 'Cargo.lock'):
            digest.update(str(path.relative_to(repo)).encode())
            digest.update(path.read_bytes())
    for name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_INCREMENTAL'):
        digest.update(name.encode() + b'=' + os.environ.get(name, '').encode())
    return digest.hexdigest()[:24]


def default_cache_root(repo):
    if os.environ.get('DANDRUM_CARGO_CACHE_ROOT'):
        return Path(os.environ['DANDRUM_CARGO_CACHE_ROOT']).absolute()
    common = subprocess.run(
        ['git', '-C', str(repo), 'rev-parse', '--path-format=absolute', '--git-common-dir'],
        capture_output=True, text=True)
    if common.returncode == 0:
        return Path(common.stdout.strip()) / 'dandrum-cargo-cache'
    return repo / 'build/cargo-cache'


def cargo_environment(generation, arguments, lane):
    # Cargo command-line configuration outranks our environment. Refuse an
    # escape from the owned storage boundary instead of silently missing it.
    if any(arg == '--config' or arg.startswith(('--config=', '--build-dir', '+')) for arg in arguments):
        raise CacheError('cache-options: toolchain/config/build-dir overrides require a separately managed invocation')
    def option(name):
        value = None
        for index, argument in enumerate(arguments):
            if argument == name:
                if index + 1 == len(arguments):
                    raise CacheError(f'cache-options: {name} needs a value')
                value = arguments[index + 1]
            elif argument.startswith(name + '='):
                value = argument.split('=', 1)[1]
        return value
    manifest = Path.cwd() / 'Cargo.toml'
    if option('--manifest-path'):
        manifest = Path(option('--manifest-path')).absolute()
    owner = hashlib.sha256(str(manifest).encode()).hexdigest()[:24]
    target = generation / 'targets' / owner / lane
    if os.environ.get('CARGO_TARGET_DIR'):
        target = Path(os.path.abspath(os.environ['CARGO_TARGET_DIR']))
    if option('--target-dir'):
        target = Path(os.path.abspath(option('--target-dir')))
    data = generation_data(generation)[lane]
    environment = dict(os.environ, CARGO_BUILD_BUILD_DIR=str(data),
                CARGO_TARGET_DIR=str(target), DANDRUM_CARGO_CACHE_ROOT=str(generation.parent.parent))
    # The launcher does not inherit the parent's Make jobserver descriptors.
    # Cargo owns its own jobserver and the maintained builds select --jobs 2.
    for name in ('MAKEFLAGS', 'MFLAGS', 'CARGO_MAKEFLAGS'):
        environment.pop(name, None)
    if lane == 'coverage':
        environment.update(CARGO_LLVM_COV_BUILD_DIR=str(data), CARGO_LLVM_COV_TARGET_DIR=str(target))
    return environment


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--cache-root', type=Path)
    parser.add_argument('--cargo', type=Path, default=Path.home() / '.cargo/bin/cargo')
    parser.add_argument('--lock-timeout', type=float, default=600)
    parser.add_argument('--ordinary', type=Path)
    parser.add_argument('--coverage', type=Path)
    parser.add_argument('--lane', choices=['ordinary', 'coverage'])
    parser.add_argument('operation', choices=['cargo', 'adopt', 'run'])
    parser.add_argument('arguments', nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    arguments = args.arguments[1:] if args.arguments[:1] == ['--'] else args.arguments
    repo = Path(os.path.abspath(args.repo))
    cache = Path(os.path.abspath(args.cache_root if args.cache_root else default_cache_root(repo)))
    try:
        if args.operation == 'cargo' and (not arguments or arguments[0].startswith(('-', '+'))):
            raise CacheError('cache-options: put the Cargo subcommand first; leading global options are not supported')
        if args.operation == 'cargo' and arguments[0] not in ('build', 'check', 'clippy', 'test', 'run', 'doc', 'tree', 'metadata', 'llvm-cov'):
            raise CacheError('cache-options: this Cargo operation needs its own storage/cleanup workflow')
        require_plain_path(cache)
        cache.parent.mkdir(parents=True, exist_ok=True)
        with cache_lease(cache.parent, args.lock_timeout, cache.name + '.init-lease'):
            initialize_cache(cache)
        with cache_lease(cache, args.lock_timeout) as lease:
            initialize_cache(cache)
            generation = select_generation(cache, generation_key(repo, args.cargo))
            if args.operation == 'adopt':
                adopt(generation, repo, args.ordinary, args.coverage)
                return 0
            environment = cargo_environment(generation, arguments,
                                           args.lane or ('coverage' if arguments[:1] == ['llvm-cov'] else 'ordinary'))
            environment['DANDRUM_CARGO_CACHE_LEASE_FD'] = str(lease)
            register_output(cache, Path(environment['CARGO_TARGET_DIR']))
            budget, before = admit(repo, cache, generation)
            record = json.loads((generation / 'owner.json').read_text())
            record['last_used_ns'] = time.time_ns()
            write_json(generation / 'owner.json', record)
            active = generation / 'active.json'
            write_json(active, {'pid': os.getpid(), 'started': time.time(),
                               'command_sha256': hashlib.sha256(json.dumps(arguments).encode()).hexdigest()})
            command = [str(args.cargo), *arguments] if args.operation == 'cargo' else arguments
            process = subprocess.Popen(command,
                                       env=environment,
                                       pass_fds=(lease,))
            result = process.wait()
            if result >= 0 and not (args.operation == 'run' and result == 125):
                active.unlink()
            after = retained_bytes(cache)
            write_json(cache / 'last-run.json', {'generation': generation.name, 'exit_status': result,
                       'retained_before_bytes': before, 'retained_after_bytes': after, 'budget_bytes': budget})
            if after > budget:
                print(f'cargo-cache: retention-overrun: retained={after}, budget={budget}; '
                      'current warm state preserved, further growth blocked', file=sys.stderr)
                return result or 2
            return result
    except (CacheError, OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        print(f'cargo-cache: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
