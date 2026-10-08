"""The optional virtual display must retain real Slint input checks."""
from pathlib import Path
import importlib.util
import io
import subprocess
import json
import os
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from slint_headless import prepare_headless, use_environment


class HeadlessLaunchTest(unittest.TestCase):
    def prepare(self, args, tools=None, environment=None, platform='linux'):
        return prepare_headless(args, environment or {}, which=lambda name: (tools or {}).get(name),
                                platform=platform, script='/repo/check-slint-library.py')

    def test_visible_and_compile_only_do_not_need_virtual_display_dependencies(self):
        self.assertIsNone(self.prepare(['--fixture', 'Controls']))
        self.assertIsNone(self.prepare(['--headless', '--check-only']))

    def test_wrapper_uses_private_display_preserves_scope_renderer_and_screenshots(self):
        args = ['--headless', '--viewer', '/tools/viewer', '--fixture', 'Controls', '--evidence', '/tmp/screenshots']
        plan = self.prepare(args, {'xvfb-run': '/tools/xvfb-run', 'Xvfb': '/tools/Xvfb', 'xauth': '/tools/xauth'},
                            {'DISPLAY': ':0', 'WAYLAND_DISPLAY': 'wayland-0', 'SLINT_BACKEND': 'winit-skia-software'})
        self.assertEqual(plan.command[:2], ['/tools/xvfb-run', '-a'])
        self.assertEqual(plan.command[-len(args[1:]):], args[1:])
        self.assertIn(sys.executable, plan.command)
        self.assertNotIn('--check-only', plan.command)
        self.assertNotIn('DISPLAY', plan.environment)
        self.assertNotIn('WAYLAND_DISPLAY', plan.environment)
        self.assertEqual(plan.environment['SLINT_BACKEND'], 'winit-skia-software')
        self.assertEqual(plan.environment['DANDRUM_SLINT_HEADLESS'], '1')

    def test_direct_xvfb_uses_existing_authenticated_viewer_lifecycle(self):
        plan = self.prepare(['--headless'], {'Xvfb': '/tools/Xvfb'}, {'DISPLAY': ':0', 'WAYLAND_DISPLAY': 'wayland-0'})
        self.assertIsNone(plan.command)
        self.assertEqual(plan.environment['DANDRUM_XVFB'], '/tools/Xvfb')
        self.assertNotIn('DISPLAY', plan.environment)
        self.assertNotIn('WAYLAND_DISPLAY', plan.environment)

    def test_explicit_xvfb_path_is_resolved_and_honored(self):
        plan = self.prepare(['--headless'], {'/custom/Xvfb': '/custom/Xvfb'}, {'DANDRUM_XVFB': '/custom/Xvfb'})
        self.assertEqual(plan.environment['DANDRUM_XVFB'], '/custom/Xvfb')
        self.assertIsNone(plan.command)

    def test_missing_xvfb_cannot_fall_back_to_visible_desktop(self):
        with self.assertRaisesRegex(RuntimeError, 'Xvfb.*DANDRUM_XVFB'):
            self.prepare(['--headless'], environment={'DISPLAY': ':0'})

    def test_environment_is_unchanged_in_visible_mode_and_restored_after_failure(self):
        before = dict(os.environ)
        with use_environment(None):
            self.assertEqual(dict(os.environ), before)
        plan = self.prepare(["--headless"], {"Xvfb": "/tools/Xvfb"})
        with self.assertRaisesRegex(ValueError, "injected failure"):
            with use_environment(plan):
                self.assertNotIn("DISPLAY", os.environ)
                raise ValueError("injected failure")
        self.assertEqual(dict(os.environ), before)

    def test_wrapper_without_xauth_falls_back_to_authenticated_direct_server(self):
        plan = self.prepare(["--headless"], {"Xvfb": "/tools/Xvfb", "xvfb-run": "/tools/xvfb-run"})
        self.assertIsNone(plan.command)
        self.assertEqual(plan.environment["DANDRUM_XVFB"], "/tools/Xvfb")

    def test_non_linux_headless_reports_supported_platform(self):
        with self.assertRaisesRegex(RuntimeError, 'Linux'):
            self.prepare(['--headless'], platform='win32')


class RunnerHeadlessTest(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('slint_runner', ROOT / 'scripts/check-slint-library.py')
        self.runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.runner)

    def test_headless_direct_mode_still_executes_contracts_input_and_screenshots(self):
        from types import SimpleNamespace
        calls = []
        class Session:
            def __init__(self, *args, **kwargs):
                calls.append(('display', os.environ.get('DISPLAY'), os.environ.get('DANDRUM_XVFB')))
            def __enter__(self): return self
            def __exit__(self, *args): calls.append(('closed',))
            def call(self, name, **kwargs): calls.append((name,))
            def find(self, name): return name
            def properties(self, handle): return {'accessibleLabel': 'PASS: contracts'}
            def wait_until(self, condition): assert condition()
            def screenshot(self, name): calls.append(('screenshot', name))
        with tempfile.TemporaryDirectory() as directory:
            env = dict(os.environ); env.pop('DISPLAY', None); env.pop('WAYLAND_DISPLAY', None)
            env.update(DANDRUM_XVFB='/tools/Xvfb', DANDRUM_SLINT_HEADLESS='1')
            plan = SimpleNamespace(command=None, environment=env)
            argv = ['runner', '--headless', '--fixture', 'Controls', '--evidence', directory]
            with patch.object(sys, 'argv', argv), patch.object(self.runner, 'prepare_headless', return_value=plan), \
                 patch.object(self.runner.shutil, 'which', return_value='/tools/viewer'), \
                 patch.object(self.runner.subprocess, 'check_output', return_value='slint-viewer 1.18.1'), \
                 patch.object(self.runner.subprocess, 'run'), patch.object(self.runner, 'ViewerSession', Session), \
                 patch.dict(self.runner.LIVE_CHECKS, {'Controls': lambda session: calls.append(('input',)) or 'PASS: real input'}, clear=True):
                self.assertEqual(self.runner.main(), 0)
            receipt = json.loads((Path(directory) / 'result.json').read_text())
            self.assertEqual(receipt['mode'], 'live-interactions')
            self.assertTrue(receipt['headless'])
            self.assertEqual(len(receipt['contracts']), 1)
            self.assertEqual(len(receipt['interactions']), 1)
            self.assertIn(('input',), calls)
            self.assertIn(('screenshot', 'controls-contracts.png'), calls)
            self.assertEqual(calls.count(('closed',)), 2)
            self.assertTrue(all(call[1] is None for call in calls if call[0] == 'display'))

    def test_compile_only_with_headless_needs_no_display_and_never_runs_input(self):
        with tempfile.TemporaryDirectory() as directory:
            argv = ["runner", "--headless", "--check-only", "--fixture", "Controls", "--evidence", directory]
            with patch.object(sys, "argv", argv), \
                 patch.object(self.runner.shutil, "which", return_value="/tools/viewer"), \
                 patch.object(self.runner.subprocess, "check_output", return_value="slint-viewer 1.18.1"), \
                 patch.object(self.runner.subprocess, "run"), patch.object(self.runner, "ViewerSession") as sessions:
                self.assertEqual(self.runner.main(), 0)
            sessions.assert_not_called()
            receipt = json.loads((Path(directory) / "result.json").read_text())
            self.assertEqual(receipt["mode"], "compile-only")
            self.assertFalse(receipt["headless"])
            self.assertEqual(receipt["interactions"], [])

    def test_missing_dependency_is_actionable_and_cannot_launch_a_visible_viewer(self):
        with patch.object(sys, 'argv', ['runner', '--headless']), \
             patch.object(self.runner, 'prepare_headless', side_effect=RuntimeError('Xvfb missing; set DANDRUM_XVFB')), \
             patch.object(self.runner, 'ViewerSession') as viewer, patch('sys.stderr', new_callable=io.StringIO) as error:
            with self.assertRaises(SystemExit) as failure: self.runner.main()
            self.assertEqual(failure.exception.code, 2)
            self.assertIn('DANDRUM_XVFB', error.getvalue())
            viewer.assert_not_called()

    def test_private_viewer_lifecycle_closes_display_after_input_failure(self):
        session = self.runner.ViewerSession('/tools/viewer', Path('/tmp/fixture.slint'), Path('/tmp'))
        session.temporary = Mock()
        viewer, display = Mock(), Mock()
        viewer.wait.side_effect = [subprocess.TimeoutExpired('viewer', 5), None]
        viewer_log, display_log = io.StringIO(), io.StringIO()
        session.processes = [(display, display_log, None), (viewer, viewer_log, None)]
        session.__exit__(AssertionError, AssertionError('input failed'), None)
        viewer.terminate.assert_called_once(); viewer.kill.assert_called_once()
        display.terminate.assert_called_once(); display.wait.assert_called_once_with(timeout=5)
        self.assertTrue(viewer_log.closed and display_log.closed)
        session.temporary.cleanup.assert_called_once()

    def test_wrapper_returns_failure_and_forwards_all_requested_checks(self):
        from types import SimpleNamespace
        plan = SimpleNamespace(command=['xvfb-run', '-a', 'python', 'runner', '--fixture', 'Controls'], environment={})
        with patch.object(sys, 'argv', ['runner', '--headless']), \
             patch.object(self.runner, 'prepare_headless', return_value=plan), \
             patch.object(self.runner.subprocess, 'run', return_value=SimpleNamespace(returncode=7)) as run:
            self.assertEqual(self.runner.main(), 7)
            run.assert_called_once_with(plan.command, env=plan.environment)


if __name__ == '__main__': unittest.main()
