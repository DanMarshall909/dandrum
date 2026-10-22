#!/usr/bin/env python3
"""Run a Linux verification command with owned scratch and retained evidence."""
import argparse
import ctypes
from contextlib import ExitStack
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import select
import stat
import shutil
import signal
import subprocess
import sys
import time
import uuid


class RunError(Exception):
    """The requested lifecycle cannot safely proceed."""


def allocated_bytes(fd):
    total = os.fstat(fd).st_blocks * 512
    for _, directories, files, current in os.fwalk('.', topdown=False, dir_fd=fd):
        for name in directories + files:
            total += os.stat(name, dir_fd=current, follow_symlinks=False).st_blocks * 512
    return total


def process_identity(pid):
    fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    return {'pid': pid, 'start': fields[19]}, fields[0], int(fields[1])


def owned_guardians(token):
    marker = ('DANDRUM_VERIFICATION_GUARD=' + token).encode()
    workers = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        fd = None
        try:
            if proc.stat().st_uid != os.getuid():
                continue
            identity, state, _ = process_identity(int(proc.name))
            if state == 'Z' or marker not in (proc / 'environ').read_bytes().split(b'\0'):
                continue
            fd = os.pidfd_open(identity['pid'])
            if process_identity(identity['pid'])[0] == identity:
                workers.append((identity, fd))
                fd = None
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue  # No unverified PID is signalled.
        finally:
            if fd is not None:
                os.close(fd)
    return workers


def reap_children():
    outcomes = []
    while True:
        try:
            # Linux __WALL includes children with nonstandard clone exit signals.
            pid, status = os.waitpid(-1, os.WNOHANG | 0x40000000)
        except ChildProcessError:
            return outcomes, True
        if pid == 0:
            return outcomes, False
        outcomes.append((pid, os.waitstatus_to_exitcode(status)))


def stop_children(observe, stopped):
    for sig, seconds in ((signal.SIGTERM, 0.25), (signal.SIGKILL, 1.0)):
        deadline = time.monotonic() + seconds
        while True:
            outcomes, empty = reap_children()
            observe(outcomes)
            children = Path('/proc/self/task/' + str(os.getpid()) + '/children').read_text().split()
            if empty and not children:
                return list(stopped.values())
            for child in children:
                fd = None
                try:
                    identity, _, parent = process_identity(int(child))
                    fd = os.pidfd_open(identity['pid'])
                    current, _, current_parent = process_identity(identity['pid'])
                    if current != identity or parent != os.getpid() or current_parent != os.getpid():
                        raise RunError('child-identity: scratch preserved')
                    signal.pidfd_send_signal(fd, sig)
                    stopped[(identity['pid'], identity['start'])] = identity
                except (FileNotFoundError, ProcessLookupError):
                    pass  # Reap the exited child on the next iteration.
                finally:
                    if fd is not None:
                        os.close(fd)
            if time.monotonic() >= deadline:
                break
            time.sleep(0.01)
    raise RunError('workers-still-running: guardian retains ownership')


def guardian(directory):
    owner = validate_owner(directory)
    if (directory / 'cleanup.json').exists():
        return 0
    stopping = [False]
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda signum, frame: stopping.__setitem__(0, True))
    signal.signal(signal.SIGCHLD, signal.SIG_DFL)
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
    libc.prctl.restype = ctypes.c_int
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER, linux/prctl.h
        raise RunError(f'subreaper-unavailable: errno {ctypes.get_errno()}')
    identity = process_identity(os.getpid())[0]
    worker = {'token': owner['token'], 'identity': identity}
    write_json(directory / 'worker.json', worker)
    process = None
    result = {'command_exit': None, 'launch_error': None}
    try:
        creator, state, _ = process_identity(owner['controller']['pid'])
        creator_alive = creator == owner['controller'] and state != 'Z'
    except (FileNotFoundError, ProcessLookupError):
        creator_alive = False
    if not creator_alive or stopping[0]:
        result['launch_error'] = 'controller-lost-before-command'
        stopping[0] = True
    else:
        environment = dict(os.environ)
        environment.pop('DANDRUM_VERIFICATION_GUARD', None)
        try:
            process = subprocess.Popen(owner['command'], cwd=owner['cwd'], env=environment)
        except OSError as error:
            result['launch_error'] = str(error)
    if process is None:
        write_json(directory / 'command.json', result)

    def observe(outcomes):
        for pid, code in outcomes:
            if process is not None and pid == process.pid:
                # This is the sole wait authority for the command. Popen.poll()
                # would turn an already-reaped exit 23 into a false exit zero.
                process.returncode = code
                result['command_exit'] = code
                write_json(directory / 'command.json', result)

    stopped_across_attempts = {}
    while True:
        outcomes, _ = reap_children()
        observe(outcomes)
        if stopping[0]:
            try:
                stopped = stop_children(observe, stopped_across_attempts)
            except (RunError, OSError) as error:
                write_json(directory / 'guardian-error.json', {
                    'error': str(error), 'stopped_workers': list(stopped_across_attempts.values())})
                stopping[0] = False  # No respawn; retain ownership for a later stop retry.
            else:
                write_json(directory / 'guardian-proof.json', {
                    **worker, **result, 'stopped_workers': stopped, 'all_children_reaped': True})
                return 0
        time.sleep(0.01)


def stop_workers(directory, token):
    workers = owned_guardians(token)
    try:
        if len(workers) > 1:
            raise RunError('guardian-identity: ambiguous guardians; scratch preserved')
        if workers:
            identity, fd = workers[0]
            deadline = time.monotonic() + 2
            while not (directory / 'worker.json').exists() and time.monotonic() < deadline:
                time.sleep(0.01)
            if not (directory / 'worker.json').exists():
                raise RunError('guardian-not-ready: scratch preserved')
            worker = json.loads((directory / 'worker.json').read_text())
            if worker != {'identity': identity, 'token': token}:
                raise RunError('guardian-identity: worker record changed')
            try:
                signal.pidfd_send_signal(fd, signal.SIGTERM)
            except ProcessLookupError:
                pass
            poll = select.poll()
            poll.register(fd, select.POLLIN)
            if not poll.poll(4000):
                raise RunError('guardian-stop-pending: scratch preserved')
        worker_path = directory / 'worker.json'
        proof_path = directory / 'guardian-proof.json'
        if not worker_path.exists() and not (directory / 'guardian-intent.json').exists():
            return []  # No guardian launch was attempted.
        if not worker_path.is_file() or not proof_path.is_file():
            raise RunError('guardian-proof-missing: scratch preserved')
        worker = json.loads(worker_path.read_text())
        proof = json.loads(proof_path.read_text())
        if (worker.get('token') != token or proof.get('token') != token
                or proof.get('identity') != worker.get('identity')
                or proof.get('all_children_reaped') is not True):
            raise RunError('guardian-proof-invalid: scratch preserved')
        return proof['stopped_workers']
    finally:
        for _, fd in workers:
            os.close(fd)


def file_identity(path):
    stat = path.lstat()
    return [stat.st_dev, stat.st_ino]


def context():
    return {'uid': os.getuid(),
            'boot_id': Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
            'pid_namespace': os.readlink('/proc/self/ns/pid')}


def write_json(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    with temporary.open('w') as output:
        output.write(json.dumps(value, indent=2) + '\n')
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, path)


def validate_owner(directory):
    owner_path = directory / 'owner.json'
    checksum_path = directory / 'owner.sha256'
    if (directory.is_symlink() or owner_path.is_symlink() or checksum_path.is_symlink()
            or (directory / 'lock').is_symlink()):
        raise RunError('owner-path: refusing a symlink')
    raw = owner_path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != checksum_path.read_text().strip():
        raise RunError('owner-checksum: ownership record changed')
    try:
        owner = json.loads(raw)
    except ValueError as error:
        raise RunError('owner-invalid: malformed record') from error
    if (not isinstance(owner, dict) or owner.get('schema') != 1
            or owner.get('owner') != 'run-owned-verification.py'
            or owner.get('directory') != str(directory)
            or owner.get('directory_identity') != file_identity(directory)
            or not isinstance(owner.get('token'), str)
            or re.fullmatch('[0-9a-f]{32}', owner['token']) is None):
        raise RunError('owner-invalid: unrecognized run identity')
    if any(owner.get(key) != value for key, value in context().items()):
        raise RunError('owner-context: recovery requires the same user, boot and PID namespace')
    scratch = directory / 'scratch'
    if os.path.lexists(scratch):
        if scratch.is_symlink() or file_identity(scratch) != owner.get('scratch_identity'):
            raise RunError('scratch-identity: replaced scratch is preserved')
    else:
        progress = directory / 'progress.json'
        if not progress.is_file() or json.loads(progress.read_text()).get('phase') != 'removing':
            raise RunError('scratch-identity: unrecorded missing scratch')
    return owner


def lock_run(lock):
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        raise RunError('run-active: controller or recovery still holds the run') from error


def finish(directory, owner, command_exit, launch_error, cancelled, stopped, recovered):
    validate_owner(directory)
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    with ExitStack() as descriptors:
        root_fd = os.open(directory, flags)
        descriptors.callback(os.close, root_fd)
        root_stat = os.fstat(root_fd)
        if [root_stat.st_dev, root_stat.st_ino] != owner['directory_identity']:
            raise RunError('owner-identity: replaced run directory is preserved')
        pinned = Path('/proc/self/fd/' + str(root_fd))
        try:
            scratch_fd = os.open('scratch', flags, dir_fd=root_fd)
        except FileNotFoundError:
            scratch_fd = None
            allocated = json.loads((pinned / 'progress.json').read_text())['scratch_allocated_bytes']
        else:
            descriptors.callback(os.close, scratch_fd)
            scratch_stat = os.fstat(scratch_fd)
            if [scratch_stat.st_dev, scratch_stat.st_ino] != owner['scratch_identity']:
                raise RunError('scratch-identity: opened scratch was replaced')
            allocated = allocated_bytes(scratch_fd)
        progress_path = pinned / 'progress.json'
        if progress_path.exists():
            previous_progress = json.loads(progress_path.read_text())
            if previous_progress.get('phase') == 'removing':
                allocated = previous_progress['scratch_allocated_bytes']
        receipt = {'command_exit': command_exit, 'launch_error': launch_error,
                   'cancel_signal': cancelled, 'stopped_workers': stopped,
                   'scratch_removed': True, 'scratch_allocated_bytes': allocated,
                   'remaining_workers': [], 'recovered': recovered,
                   'free_before_bytes': owner['free_before_bytes']}
        write_json(pinned / 'progress.json', {**receipt, 'phase': 'removing'})
        if scratch_fd is not None:
            # fwalk pins each directory; unlink/rmdir are relative to those
            # descriptors and never follow a resource/cache symlink.
            for _, directories, files, current in os.fwalk('.', topdown=False, dir_fd=scratch_fd):
                for name in files:
                    os.unlink(name, dir_fd=current)
                for name in directories:
                    mode = os.stat(name, dir_fd=current, follow_symlinks=False).st_mode
                    if stat.S_ISLNK(mode):
                        os.unlink(name, dir_fd=current)
                    else:
                        os.rmdir(name, dir_fd=current)
            entry = os.stat('scratch', dir_fd=root_fd, follow_symlinks=False)
            if [entry.st_dev, entry.st_ino] != owner['scratch_identity']:
                raise RunError('scratch-identity: changed entry is preserved')
            os.rmdir('scratch', dir_fd=root_fd)
        receipt['free_after_bytes'] = shutil.disk_usage(pinned).free
        write_json(pinned / 'cleanup.json', receipt)


def run(directory, command):
    directory = directory.absolute()
    try:
        directory.mkdir(mode=0o700)
    except FileExistsError as error:
        raise RunError(f'run-directory-exists: {directory}') from error
    directory = directory.resolve()
    with (directory / 'lock').open('xb') as lock:
        lock_run(lock)
        scratch = directory / 'scratch'
        scratch.mkdir(mode=0o700)
        owner = {'schema': 1, 'owner': 'run-owned-verification.py',
                 'directory': str(directory), 'directory_identity': file_identity(directory),
                 'scratch_identity': file_identity(scratch), 'token': uuid.uuid4().hex,
                 'command': command, 'cwd': os.getcwd(),
                 'controller': process_identity(os.getpid())[0], **context(),
                 'free_before_bytes': shutil.disk_usage(directory).free}
        write_json(directory / 'owner.json', owner)
        (directory / 'owner.sha256').write_text(
            hashlib.sha256((directory / 'owner.json').read_bytes()).hexdigest() + '\n')
        command_exit = None
        launch_error = None
        cancelled = [None]

        def request_stop(signum, frame):
            cancelled[0] = signum

        previous = {sig: signal.signal(sig, request_stop) for sig in (signal.SIGINT, signal.SIGTERM)}
        try:
            with (directory / 'stdout.log').open('wb') as stdout, \
                    (directory / 'stderr.log').open('wb') as stderr:
                write_json(directory / 'guardian-intent.json', {'token': owner['token']})
                process = subprocess.Popen(
                    [sys.executable, '-B', str(Path(__file__).resolve()), '_guardian',
                     '--directory', str(directory)],
                    env=dict(os.environ, TMPDIR=str(scratch),
                             DANDRUM_VERIFICATION_SCRATCH=str(scratch),
                             DANDRUM_VERIFICATION_GUARD=owner['token']),
                    stdout=stdout, stderr=stderr, start_new_session=True, close_fds=True)
                deadline = time.monotonic() + 2
                while not (directory / 'worker.json').exists():
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise RunError('guardian-not-ready: scratch preserved')
                    time.sleep(0.01)
                while not (directory / 'command.json').exists() and cancelled[0] is None:
                    if process.poll() is not None:
                        raise RunError('guardian-proof-missing: scratch preserved')
                    time.sleep(0.01)
                validate_owner(directory)
                stopped = stop_workers(directory, owner['token'])
                process.wait(timeout=1)
                result = json.loads((directory / 'guardian-proof.json').read_text())
                command_exit = result['command_exit']
                launch_error = result['launch_error']
                if launch_error:
                    print(f'launch-failed: {launch_error}', file=sys.stderr)
            finish(directory, owner, command_exit, launch_error, cancelled[0], stopped, False)
            if cancelled[0] is not None:
                return 128 + cancelled[0]
            if command_exit is None:
                return 1
            return 128 - command_exit if command_exit < 0 else command_exit
        finally:
            for sig, handler in previous.items():
                signal.signal(sig, handler)


def recover(directory):
    directory = directory.absolute()
    owner = validate_owner(directory)
    with (directory / 'lock').open('rb') as lock:
        lock_run(lock)
        owner = validate_owner(directory)
        if (directory / 'cleanup.json').is_file() and not (directory / 'scratch').exists():
            return 0
        result_path = directory / 'command.json'
        known_result = json.loads(result_path.read_text()) if result_path.exists() else {}
        stopped = stop_workers(directory, owner['token'])
        finish(directory, owner, known_result.get('command_exit'), known_result.get('launch_error'),
               None, stopped, True)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='operation', required=True)
    execute = sub.add_parser('run')
    execute.add_argument('--directory', type=Path, required=True)
    execute.add_argument('command', nargs=argparse.REMAINDER)
    for operation in ('recover', '_guardian'):
        recovery = sub.add_parser(operation)
        recovery.add_argument('--directory', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.operation == '_guardian':
            return guardian(args.directory)
        if args.operation == 'recover':
            return recover(args.directory)
        command = args.command[1:] if args.command[:1] == ['--'] else args.command
        if not command:
            parser.error('run requires a command')
        return run(args.directory, command)
    except (RunError, OSError, ValueError) as error:
        print(f'owned-verification: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
