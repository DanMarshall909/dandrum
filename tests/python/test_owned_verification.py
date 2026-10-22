"""Linux process/filesystem contracts; no compiler or real build caches."""
import json
import hashlib
import importlib.util
import os
import select
import signal
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / 'scripts/run-owned-verification.py'


@unittest.skipUnless(sys.platform.startswith('linux'), 'Linux worker ownership boundary')
class OwnedVerificationFixture(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='owned verification ')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.run = self.root / 'run with spaces'
        self.cache = self.root / 'reusable-cache'
        self.cache.mkdir()
        (self.cache / 'sentinel').write_bytes(b'reusable dependency')

    def invoke(self, *args):
        return subprocess.run([sys.executable, '-B', str(DRIVER), *args],
                              capture_output=True, text=True, timeout=10)

    def fault_bootstrap(self, body):
        site = self.root / 'fault-site'
        site.mkdir()
        (site / 'sitecustomize.py').write_text(
            'import os,pathlib,signal\n' + body +
            # Keep the caller's tracing bootstrap active in faulted processes.
            f'for folder in {os.environ.get("PYTHONPATH", "")!r}.split(os.pathsep):\n'
            ' candidate=pathlib.Path(folder)/"sitecustomize.py"\n'
            ' if folder and candidate.is_file():\n'
            '  exec(compile(candidate.read_bytes(),str(candidate),"exec"),{})\n'
            '  break\n')
        return {'PYTHONPATH': str(site)}

    def own_guardian(self, record):
        try:
            fd = os.pidfd_open(record['pid'])
        except ProcessLookupError:
            return None  # It may already have exited and been reaped by init.
        self.addCleanup(os.close, fd)
        try:
            current = Path(f"/proc/{record['pid']}/stat").read_text().rsplit(')', 1)[1].split()
        except FileNotFoundError:
            return fd  # The pinned process has exited; teardown still closes fd.
        self.assertEqual(current[19], record['start'])

        def stop():
            try:
                signal.pidfd_send_signal(fd, signal.SIGTERM)
            except ProcessLookupError:
                pass
            poll = select.poll()
            poll.register(fd, select.POLLIN)
            self.assertTrue(poll.poll(3000), 'startup guardian retained live children')

        self.addCleanup(stop)
        return fd

    def start_workers(self, clear_environment=False, nested=False):
        # A detached descendant ignores TERM, so cleanup must find and stop it
        # rather than assuming the command leader or its process group is enough.
        child = self.root / 'descendant.py'
        child.write_text(
            'import json,os,pathlib,signal,sys,time\n'
            'signal.signal(signal.SIGTERM, signal.SIG_IGN)\n'
            'signal.signal(signal.SIGINT, signal.SIG_IGN)\n'
            's=pathlib.Path("/proc/self/stat").read_text().rsplit(")",1)[1].split()\n'
            'p=pathlib.Path(sys.argv[1]); t=p.with_suffix(".tmp"); t.write_text(json.dumps({"pid":os.getpid(),"start":s[19]})); t.replace(p)\n'
            'print("descendant ready",flush=True)\n'
            'time.sleep(60)\n')
        leader = self.root / 'leader.py'
        leader.write_text(
            'import json,os,pathlib,subprocess,sys,time\n'
            'scratch=pathlib.Path(os.environ["TMPDIR"])\n'
            '(scratch/"temporary.bin").write_bytes(b"x"*65536)\n'
            '(scratch/"shared").symlink_to(sys.argv[3],target_is_directory=True)\n'
            f'subprocess.Popen([sys.executable,sys.argv[1],sys.argv[2]],start_new_session=True{",env={}" if clear_environment else ""})\n'
            'child=pathlib.Path(sys.argv[2])\n'
            'while not child.exists():time.sleep(0.005)\n'
            's=pathlib.Path("/proc/self/stat").read_text().rsplit(")",1)[1].split()\n'
            'p=pathlib.Path(sys.argv[4]); t=p.with_suffix(".tmp"); t.write_text(json.dumps([{"pid":os.getpid(),"start":s[19]},json.loads(child.read_text())])); t.replace(p)\n'
            'print("leader ready",flush=True)\n'
            'time.sleep(60)\n')
        child_ready = self.root / 'child-ready.json'
        ready = self.root / 'workers.json'
        command = [sys.executable, str(leader), str(child), str(child_ready), str(self.cache), str(ready)]
        if nested:
            command = [sys.executable, '-c',
                       'import os,pathlib,subprocess,sys; '
                       'sys.exit(subprocess.call([sys.executable,sys.argv[1],"run","--directory",'
                       'str(pathlib.Path(os.environ["TMPDIR"])/"nested-run"),"--",*sys.argv[2:]]))',
                       str(DRIVER), *command]
        controller = subprocess.Popen(
            [sys.executable, '-B', str(DRIVER), 'run', '--directory', str(self.run), '--', *command],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        handles = []
        guardians = []

        def cleanup():
            if controller.poll() is None:
                controller.kill()
            controller.communicate(timeout=3)
            if not guardians and (self.run / 'worker.json').exists():
                identity = json.loads((self.run / 'worker.json').read_text())['identity']
                try:
                    fd = os.pidfd_open(identity['pid'])
                    current = Path(f"/proc/{identity['pid']}/stat").read_text().rsplit(')', 1)[1].split()
                    if current[19] == identity['start']:
                        guardians.append(fd)
                    else:
                        os.close(fd)
                except ProcessLookupError:
                    pass
            for fd in guardians:
                try:
                    signal.pidfd_send_signal(fd, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                poll = select.poll()
                poll.register(fd, select.POLLIN)
                self.assertTrue(poll.poll(3000), 'test guardian did not stop its owned children')
                os.close(fd)
            # A failed readiness assertion still owns a recorded descendant.
            records = json.loads(ready.read_text()) if ready.exists() else (
                [json.loads(child_ready.read_text())] if child_ready.exists() else [])
            for record in records:
                if any(pid == record['pid'] for pid, _ in handles):
                    continue
                try:
                    fd = os.pidfd_open(record['pid'])
                    stat = Path(f"/proc/{record['pid']}/stat").read_text().rsplit(')', 1)[1].split()
                    if stat[19] == record['start']:
                        handles.append((record['pid'], fd))
                    else:
                        os.close(fd)
                except ProcessLookupError:
                    pass
            for _, fd in handles:
                try:
                    signal.pidfd_send_signal(fd, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                os.close(fd)

        self.addCleanup(cleanup)
        for _ in range(200):
            if ready.exists():
                break
            if controller.poll() is not None:
                self.fail(f'controller exited before workers were ready: {controller.communicate()}')
            time.sleep(0.01)
        self.assertTrue(ready.exists(), 'owned workers did not report readiness')
        for record in json.loads(ready.read_text()):
            fd = os.pidfd_open(record['pid'])
            stat = Path(f"/proc/{record['pid']}/stat").read_text().rsplit(')', 1)[1].split()
            self.assertEqual(stat[19], record['start'])
            handles.append((record['pid'], fd))
        identity = json.loads((self.run / 'worker.json').read_text())['identity']
        fd = os.pidfd_open(identity['pid'])
        self.assertEqual(Path(f"/proc/{identity['pid']}/stat").read_text().rsplit(')', 1)[1].split()[19], identity['start'])
        guardians.append(fd)
        return controller, handles

    def assert_workers_stopped(self, handles):
        for pid, fd in handles:
            poll = select.poll()
            poll.register(fd, select.POLLIN)
            self.assertTrue(poll.poll(1000), f'owned worker {pid} remains alive')



class OwnedVerificationTest(OwnedVerificationFixture):
    def test_controller_death_before_guardian_startup_cannot_launch_command(self):
        ledger = self.root / 'guardian-started.json'
        forbidden = self.root / 'command-must-not-run'
        environment = self.fault_bootstrap(
            'if os.environ.get("DANDRUM_VERIFICATION_GUARD"):\n'
            ' import json,sys\n'
            ' directory=pathlib.Path(sys.argv[-1])\n'
            ' owner=json.loads((directory/"owner.json").read_text())\n'
            ' creator=owner["controller"]\n'
            ' fd=os.pidfd_open(creator["pid"])\n'
            ' current=pathlib.Path("/proc/"+str(creator["pid"])+"/stat").read_text().rsplit(")",1)[1].split()\n'
            ' assert current[19]==creator["start"]\n'
            ' own=pathlib.Path("/proc/self/stat").read_text().rsplit(")",1)[1].split()\n'
            f' pathlib.Path({str(ledger)!r}).write_text(json.dumps({{"pid":os.getpid(),"start":own[19]}}))\n'
            ' signal.pidfd_send_signal(fd,signal.SIGKILL);os.close(fd)\n')
        with patch.dict(os.environ, environment):
            result = self.invoke('run', '--directory', str(self.run), '--', sys.executable,
                                 '-c', 'import pathlib,sys;pathlib.Path(sys.argv[1]).touch()', str(forbidden))
        self.assertEqual(result.returncode, -signal.SIGKILL, result.stderr)
        self.assertTrue(ledger.is_file(), 'pre-start controller death fault was not applied')
        guardian_fd = self.own_guardian(json.loads(ledger.read_text()))
        if guardian_fd is not None:
            poll = select.poll()
            poll.register(guardian_fd, select.POLLIN)
            self.assertTrue(poll.poll(3000), 'guardian did not close after losing its creator')
        self.assertFalse(forbidden.exists())
        proof = json.loads((self.run / 'guardian-proof.json').read_text())
        self.assertTrue(proof['all_children_reaped'])
        self.assertEqual(proof['launch_error'], 'controller-lost-before-command')
        self.assertTrue((self.run / 'scratch').exists())
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.run / 'scratch').exists())

    def test_stop_crossing_launch_window_reaps_admitted_command_before_cleanup(self):
        ledger = self.root / 'launched-child.json'
        environment = self.fault_bootstrap(
            'if os.environ.get("DANDRUM_VERIFICATION_GUARD"):\n'
            ' import json,subprocess\n'
            ' original=subprocess.Popen\n'
            ' def launch(*args,**kwargs):\n'
            '  os.kill(os.getpid(),signal.SIGTERM)\n'
            '  child=original(*args,**kwargs)\n'
            f'  pathlib.Path({str(ledger)!r}).write_text(json.dumps({{"pid":child.pid}}))\n'
            '  return child\n'
            ' subprocess.Popen=launch\n')
        with patch.dict(os.environ, environment):
            result = self.invoke('run', '--directory', str(self.run), '--',
                                 sys.executable, '-c', 'import time;time.sleep(60)')
        self.assertEqual(result.returncode, 128 + signal.SIGTERM, result.stderr)
        self.assertTrue(ledger.is_file(), 'stop-at-launch fault did not admit a child')
        child = json.loads(ledger.read_text())['pid']
        proof = json.loads((self.run / 'guardian-proof.json').read_text())
        self.assertTrue(proof['all_children_reaped'])
        self.assertEqual(proof['command_exit'], -signal.SIGTERM)
        self.assertIn(child, [worker['pid'] for worker in proof['stopped_workers']])
        self.assertFalse(Path(f'/proc/{child}').exists(), 'admitted child was not reaped')
        self.assertFalse((self.run / 'scratch').exists())

    def test_cancellation_stops_command_and_detached_descendant_before_cleanup(self):
        controller, handles = self.start_workers()
        controller.send_signal(signal.SIGINT)
        stdout, stderr = controller.communicate(timeout=3)
        self.assertEqual(controller.returncode, 130, stderr)
        self.assert_workers_stopped(handles)
        self.assertFalse((self.run / 'scratch').exists())
        receipt = json.loads((self.run / 'cleanup.json').read_text())
        self.assertEqual(receipt['cancel_signal'], signal.SIGINT)
        self.assertEqual(receipt['remaining_workers'], [])
        self.assertGreaterEqual(len(receipt['stopped_workers']), 2)
        self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')
        self.assertIn('descendant ready', (self.run / 'stdout.log').read_text())

    def test_cancellation_also_stops_detached_child_that_clears_environment(self):
        controller, handles = self.start_workers(clear_environment=True)
        controller.send_signal(signal.SIGINT)
        _, stderr = controller.communicate(timeout=3)
        self.assertEqual(controller.returncode, 130, stderr)
        self.assert_workers_stopped(handles)
        self.assertFalse((self.run / 'scratch').exists())
        self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')

    def test_recovery_after_controller_death_stops_recorded_workers_and_cleans_once(self):
        controller, handles = self.start_workers()
        controller.kill()
        controller.communicate(timeout=3)
        self.assertTrue((self.run / 'scratch').exists())
        # The command and its detached descendant outlive SIGKILL of the driver.
        for _, fd in handles:
            poll = select.poll()
            poll.register(fd, select.POLLIN)
            self.assertEqual(poll.poll(0), [])
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_workers_stopped(handles)
        self.assertFalse((self.run / 'scratch').exists())
        receipt_bytes = (self.run / 'cleanup.json').read_bytes()
        receipt = json.loads(receipt_bytes)
        self.assertTrue(receipt['recovered'])
        self.assertIsNone(receipt['command_exit'])
        self.assertEqual(receipt['remaining_workers'], [])
        self.assertGreaterEqual(len(receipt['stopped_workers']), 2)
        self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')
        self.assertIn('leader ready', (self.run / 'stdout.log').read_text())
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.run / 'cleanup.json').read_bytes(), receipt_bytes)

    def test_live_controller_refuses_concurrent_recovery(self):
        controller, handles = self.start_workers()
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('run-active', result.stderr)
        self.assertTrue((self.run / 'scratch').exists())
        self.assertIsNone(controller.poll())
        controller.send_signal(signal.SIGTERM)
        controller.communicate(timeout=3)
        self.assertEqual(controller.returncode, 143)
        self.assert_workers_stopped(handles)

    def test_invalid_owner_context_and_replaced_scratch_refuse_cleanup(self):
        controller, handles = self.start_workers()
        owner = self.run / 'owner.json'
        checksum = self.run / 'owner.sha256'
        self.assertTrue(owner.is_file(), 'a recoverable run must retain its exact ownership record')
        original = owner.read_bytes()
        original_checksum = checksum.read_bytes()
        controller.kill()
        controller.communicate(timeout=3)
        scratch = self.run / 'scratch'
        cases = [('corrupt-owner', b'broken', 'owner-checksum'),
                 ('malformed-owner', b'{', 'owner-invalid'),
                 ('unknown-owner', None, 'owner-invalid'),
                 ('wrong-context', None, 'owner-context'),
                 ('changed-scratch', None, 'scratch-identity')]
        for kind, bad_bytes, diagnostic in cases:
            with self.subTest(kind=kind):
                if kind == 'corrupt-owner':
                    owner.write_bytes(bad_bytes)
                elif kind == 'malformed-owner':
                    owner.write_bytes(bad_bytes)
                    checksum.write_text(hashlib.sha256(owner.read_bytes()).hexdigest() + '\n')
                elif kind == 'unknown-owner':
                    metadata = json.loads(original)
                    metadata['schema'] = 99
                    owner.write_text(json.dumps(metadata))
                    checksum.write_text(hashlib.sha256(owner.read_bytes()).hexdigest() + '\n')
                elif kind == 'wrong-context':
                    metadata = json.loads(original)
                    metadata['pid_namespace'] = 'different-namespace'
                    owner.write_text(json.dumps(metadata))
                    checksum.write_text(hashlib.sha256(owner.read_bytes()).hexdigest() + '\n')
                else:
                    scratch.rename(self.run / 'original-scratch')
                    scratch.symlink_to(self.cache, target_is_directory=True)
                result = self.invoke('recover', '--directory', str(self.run))
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn(diagnostic, result.stderr)
                self.assertTrue(scratch.exists())
                self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')
                for _, fd in handles:
                    poll = select.poll()
                    poll.register(fd, select.POLLIN)
                    self.assertEqual(poll.poll(0), [], 'invalid ownership must not signal workers')
                owner.write_bytes(original)
                checksum.write_bytes(original_checksum)
                if kind == 'changed-scratch':
                    scratch.unlink()
                    (self.run / 'original-scratch').rename(scratch)
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_workers_stopped(handles)

    def test_completed_commands_remove_scratch_and_preserve_logs_and_shared_target(self):
        for exit_code in (0, 23):
            with self.subTest(exit_code=exit_code):
                run = self.root / f'run-{exit_code}'
                command = [sys.executable, '-c',
                           'import os,pathlib,sys; p=pathlib.Path(os.environ["TMPDIR"]); '
                           '(p/"temporary.bin").write_bytes(b"x"*65536); '
                           '(p/"nested").mkdir(); (p/"nested/leaf").write_bytes(b"nested output"); '
                           '(p/"shared").symlink_to(sys.argv[1], target_is_directory=True); '
                           'print("retained stdout"); print("retained stderr",file=sys.stderr); '
                           'sys.exit(int(sys.argv[2]))', str(self.cache), str(exit_code)]
                result = self.invoke('run', '--directory', str(run), '--', *command)
                self.assertEqual(result.returncode, exit_code, result.stderr)
                self.assertFalse((run / 'scratch').exists())
                self.assertEqual((run / 'stdout.log').read_text(), 'retained stdout\n')
                self.assertEqual((run / 'stderr.log').read_text(), 'retained stderr\n')
                receipt = json.loads((run / 'cleanup.json').read_text())
                self.assertEqual(receipt['command_exit'], exit_code)
                self.assertTrue(receipt['scratch_removed'])
                self.assertGreaterEqual(receipt['scratch_allocated_bytes'], 65536)
                self.assertEqual(receipt['remaining_workers'], [])
                self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')

    def test_launch_failure_cleans_scratch_and_retains_the_failure(self):
        result = self.invoke('run', '--directory', str(self.run), '--',
                             str(self.root / 'missing-command'))
        self.assertEqual(result.returncode, 1)
        self.assertIn('launch-failed', result.stderr)
        self.assertFalse((self.run / 'scratch').exists())
        receipt = json.loads((self.run / 'cleanup.json').read_text())
        self.assertIsNone(receipt['command_exit'])
        self.assertIn('missing-command', receipt['launch_error'])
        self.assertTrue(receipt['scratch_removed'])

    def test_signalled_command_retains_its_signal_and_conventional_exit_code(self):
        result = self.invoke('run', '--directory', str(self.run), '--', sys.executable,
                             '-c', 'import os,signal; os.kill(os.getpid(),signal.SIGUSR1)')
        self.assertEqual(result.returncode, 128 + signal.SIGUSR1, result.stderr)
        receipt = json.loads((self.run / 'cleanup.json').read_text())
        self.assertEqual(receipt['command_exit'], -signal.SIGUSR1)
        self.assertFalse((self.run / 'scratch').exists())

    def test_guardian_loss_without_proof_preserves_scratch_and_refuses_recovery(self):
        controller, handles = self.start_workers()
        identity = json.loads((self.run / 'worker.json').read_text())['identity']
        fd = os.pidfd_open(identity['pid'])
        try:
            self.assertEqual(Path(f"/proc/{identity['pid']}/stat").read_text().rsplit(')', 1)[1].split()[19], identity['start'])
            signal.pidfd_send_signal(fd, signal.SIGKILL)
        finally:
            os.close(fd)
        _, stderr = controller.communicate(timeout=3)
        self.assertEqual(controller.returncode, 1)
        self.assertIn('guardian-proof-missing', stderr)
        self.assertTrue((self.run / 'scratch').is_dir())
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 1)
        self.assertIn('guardian-proof-missing', result.stderr)
        self.assertFalse((self.run / 'cleanup.json').exists())
        self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')

    def test_scratch_replacement_after_validation_cannot_delete_unowned_contents(self):
        spec = importlib.util.spec_from_file_location('owned_verification_probe', DRIVER)
        driver = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(driver)
        original_validate = driver.validate_owner
        injected = []

        def replace_after_validation(directory):
            owner = original_validate(directory)
            if (directory / 'guardian-proof.json').exists() and not injected:
                (directory / 'scratch').rename(directory / 'original-scratch')
                self.cache.rename(directory / 'scratch')
                injected.append(True)
            return owner

        try:
            with patch.object(driver, 'validate_owner', side_effect=replace_after_validation):
                code = driver.main(['run', '--directory', str(self.run), '--',
                                    sys.executable, '-c', 'print("actual command")'])
            self.assertTrue(injected, 'scratch replacement fault was not applied')
            self.assertEqual(code, 1, 'changed scratch identity must be refused')
            self.assertEqual((self.run / 'scratch/sentinel').read_bytes(), b'reusable dependency')
            self.assertTrue((self.run / 'original-scratch').is_dir())
            self.assertFalse((self.run / 'cleanup.json').exists())
        finally:
            if (self.run / 'scratch/sentinel').exists():
                (self.run / 'scratch').rename(self.cache)

    def test_nested_run_transfers_orphans_to_outer_owner_before_removal(self):
        controller, handles = self.start_workers(clear_environment=True, nested=True)
        controller.send_signal(signal.SIGINT)
        _, stderr = controller.communicate(timeout=3)
        self.assertEqual(controller.returncode, 130, stderr)
        self.assert_workers_stopped(handles)
        self.assertFalse((self.run / 'scratch').exists())
        proof = json.loads((self.run / 'guardian-proof.json').read_text())
        self.assertTrue(proof['all_children_reaped'])
        self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')

    def test_recovery_preserves_known_outcome_and_allocation_across_deletion_crashes(self):
        shim = self.root / 'crash.py'
        shim.write_text(
            'import importlib.util,os,pathlib,sys\n'
            'spec=importlib.util.spec_from_file_location("crash_driver",sys.argv[1])\n'
            'driver=importlib.util.module_from_spec(spec);spec.loader.exec_module(driver)\n'
            'phase=sys.argv[2]\n'
            'if phase=="before-finish":\n'
            ' driver.finish=lambda *args:os._exit(99)\n'
            'elif phase=="during-delete":\n'
            ' original=driver.os.unlink\n'
            ' def unlink(name,*args,**kwargs):\n'
            '  original(name,*args,**kwargs)\n'
            '  if str(name)=="temporary.bin":os._exit(99)\n'
            ' driver.os.unlink=unlink\n'
            'else:\n'
            ' original=driver.write_json\n'
            ' def write(path,value):\n'
            '  if path.name=="cleanup.json":os._exit(99)\n'
            '  return original(path,value)\n'
            ' driver.write_json=write\n'
            'raise SystemExit(driver.main(sys.argv[3:]))\n')
        for phase in ('before-finish', 'during-delete', 'after-delete'):
            with self.subTest(phase=phase):
                run = self.root / phase
                command = [sys.executable, '-c',
                           'import os,pathlib; p=pathlib.Path(os.environ["TMPDIR"]); '
                           '(p/"temporary.bin").write_bytes(b"x"*65536); '
                           'print("retained crash evidence")']
                result = subprocess.run(
                    [sys.executable, str(shim), str(DRIVER), phase, 'run', '--directory', str(run), '--', *command],
                    capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 99, result.stderr)
                self.assertTrue(json.loads((run / 'guardian-proof.json').read_text())['all_children_reaped'])
                if phase == 'before-finish':
                    proof_path = run / 'guardian-proof.json'
                    proof_bytes = proof_path.read_bytes()
                    proof = json.loads(proof_bytes)
                    proof['all_children_reaped'] = False
                    proof_path.write_text(json.dumps(proof))
                    rejected = self.invoke('recover', '--directory', str(run))
                    self.assertEqual(rejected.returncode, 1, rejected.stderr)
                    self.assertIn('guardian-proof-invalid', rejected.stderr)
                    self.assertTrue((run / 'scratch/temporary.bin').exists())
                    proof_path.write_bytes(proof_bytes)
                progress = run / 'progress.json'
                initial_allocation = json.loads(progress.read_text())['scratch_allocated_bytes'] if progress.exists() else None
                result = self.invoke('recover', '--directory', str(run))
                self.assertEqual(result.returncode, 0, result.stderr)
                receipt_bytes = (run / 'cleanup.json').read_bytes()
                receipt = json.loads(receipt_bytes)
                if initial_allocation is not None:
                    self.assertEqual(receipt['scratch_allocated_bytes'], initial_allocation)
                self.assertEqual(receipt['command_exit'], 0)
                self.assertTrue(receipt['recovered'])
                self.assertFalse((run / 'scratch').exists())
                self.assertEqual((run / 'stdout.log').read_text(), 'retained crash evidence\n')
                self.assertEqual(self.invoke('recover', '--directory', str(run)).returncode, 0)
                self.assertEqual((run / 'cleanup.json').read_bytes(), receipt_bytes)
                self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')

    def test_supervised_source_snapshot_is_disposable_while_target_and_log_survive(self):
        checkout = self.root / 'checkout'
        checkout.mkdir()
        (checkout / 'CMakeLists.txt').write_text('# fixture source\n')
        for name in ('.githooks', 'src', 'tests', 'examples', 'schema', 'scripts', 'spikes'):
            (checkout / name).mkdir()
            (checkout / name / 'input.txt').write_text(name)
        cache = checkout / 'build/engine-only-regression/cargo-target'
        cache.mkdir(parents=True)
        (cache / 'dependency').write_bytes(b'reusable binary')
        retained = cache.parent / 'retained.log'
        retained.write_bytes(b'command evidence')
        probe = self.root / 'fixture-probe.py'
        probe.write_text(
            'import json,os,pathlib,sys,unittest\n'
            'sys.path.insert(0,sys.argv[1])\n'
            'import test_engine_only_build as fixture\n'
            'fixture.ROOT=pathlib.Path(sys.argv[2])\n'
            'expected=pathlib.Path(sys.argv[3])\n'
            'class Probe(fixture.EngineBuildFixture):\n'
            ' def test_owned(self):\n'
            '  self.assertEqual(self.source.parent.parent,expected)\n'
            '  self.assertEqual((self.source/"src/input.txt").read_text(),"src")\n'
            '  (self.source/"shared").symlink_to(self.cargo_target,target_is_directory=True)\n'
            'result=unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(Probe))\n'
            'sys.exit(not result.wasSuccessful())\n')
        result = self.invoke('run', '--directory', str(self.run), '--', sys.executable,
                             str(probe), str(ROOT / 'tests/python'), str(checkout), str(self.run / 'scratch'))
        self.assertEqual(result.returncode, 0, (self.run / 'stderr.log').read_text())
        self.assertFalse((self.run / 'scratch').exists())
        self.assertEqual(list(cache.parent.glob('fixture-*')), [])
        self.assertEqual((cache / 'dependency').read_bytes(), b'reusable binary')
        self.assertEqual(retained.read_bytes(), b'command evidence')

    def test_existing_run_paths_and_symlinks_are_preserved(self):
        for name, symlink in [('directory', False), ('symlink', True)]:
            with self.subTest(kind=name):
                run = self.root / name
                if symlink:
                    run.symlink_to(self.cache, target_is_directory=True)
                else:
                    run.mkdir()
                    (run / 'sentinel').write_bytes(b'prior run evidence')
                result = self.invoke('run', '--directory', str(run), '--',
                                     sys.executable, '-c', 'raise AssertionError("must not run")')
                self.assertEqual(result.returncode, 1)
                self.assertIn('run-directory-exists', result.stderr)
                self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')
                if not symlink:
                    self.assertEqual((run / 'sentinel').read_bytes(), b'prior run evidence')


class OwnedVerificationRetryTest(OwnedVerificationFixture):
    """Slow lane: observes the real four-second pending-stop deadline."""

    def test_failed_stop_retry_retains_earlier_stopped_workers(self):
        applied = self.root / 'stop-fault-applied'
        environment = self.fault_bootstrap(
            f'fault=pathlib.Path({str(applied)!r})\n'
            'if os.environ.get("DANDRUM_VERIFICATION_GUARD"):\n'
            ' original=signal.pidfd_send_signal\n'
            ' def send(fd,sig,*args):\n'
            '  if sig==signal.SIGKILL and not fault.exists():\n'
            '   fault.write_text("injected stop failure")\n'
            '   raise PermissionError("injected stop failure")\n'
            '  return original(fd,sig,*args)\n'
            ' signal.pidfd_send_signal=send\n')
        with patch.dict(os.environ, environment):
            controller, handles = self.start_workers(clear_environment=True)
        controller.send_signal(signal.SIGINT)
        _, stderr = controller.communicate(timeout=6)
        self.assertEqual(controller.returncode, 1, stderr)
        self.assertIn('guardian-stop-pending', stderr)
        self.assertTrue(applied.exists(), 'stop failure fault was not applied')
        self.assertTrue((self.run / 'scratch').is_dir())
        self.assertFalse((self.run / 'cleanup.json').exists())
        self.assertIn('injected stop failure', (self.run / 'guardian-error.json').read_text())
        root_poll = select.poll()
        root_poll.register(handles[0][1], select.POLLIN)
        self.assertTrue(root_poll.poll(0), 'the first attempt must stop the command leader')
        child_poll = select.poll()
        child_poll.register(handles[1][1], select.POLLIN)
        self.assertEqual(child_poll.poll(0), [], 'the failed KILL must leave the descendant owned and alive')
        result = self.invoke('recover', '--directory', str(self.run))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_workers_stopped(handles)
        receipt = json.loads((self.run / 'cleanup.json').read_text())
        self.assertTrue({pid for pid, _ in handles}.issubset(
            {worker['pid'] for worker in receipt['stopped_workers']}),
            'retry receipt must retain workers stopped by the earlier attempt')
        self.assertFalse((self.run / 'scratch').exists())
        self.assertEqual((self.cache / 'sentinel').read_bytes(), b'reusable dependency')


if __name__ == '__main__':
    unittest.main()
