import importlib.util
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch, MagicMock

ROOT = Path(__file__).resolve().parents[2]


class NativeSlintTestRunnerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location('native_slint_test', ROOT / 'scripts/run-native-slint-test.py')
        cls.runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.runner)

    def run_case(self, desktop=False, result=0, error=None):
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory) / 'evidence'
            args = SimpleNamespace(executable=Path('/native/test'), evidence=evidence)
            session = MagicMock()
            environment = {'DISPLAY': ':0'} if desktop else {'DANDRUM_XVFB': '/private/Xvfb'}
            with patch.object(self.runner, 'ViewerSession', return_value=session), \
                 patch.dict(self.runner.os.environ, environment, clear=True), \
                 patch.object(self.runner.subprocess, 'run', side_effect=error,
                              return_value=SimpleNamespace(returncode=result)) as execute:
                if error:
                    with self.assertRaises(type(error)): self.runner.run_test(args)
                else:
                    self.assertEqual(self.runner.run_test(args), result)
                self.assertEqual(session.start_virtual_display.call_count, 0 if desktop else 1)
                session.__exit__.assert_called_once_with(None, None, None)
                execute.assert_called_once()
                self.assertEqual(execute.call_args.args[0], ['/native/test', str(evidence)])
                self.assertEqual(execute.call_args.kwargs['env'], session.env)
                self.assertEqual(execute.call_args.kwargs['timeout'], 90)
                self.assertTrue((evidence / 'native-test.log').exists())

    def test_no_display_uses_private_server_and_releases_it(self): self.run_case()
    def test_desktop_option_runs_same_renderer_test(self): self.run_case(desktop=True)
    def test_native_failure_is_returned_instead_of_reported_as_green(self): self.run_case(result=1)
    def test_timeout_still_releases_private_server(self):
        self.run_case(error=subprocess.TimeoutExpired('/native/test', 90))

    def test_failed_display_start_still_releases_owned_resources(self):
        with tempfile.TemporaryDirectory() as directory:
            session = MagicMock()
            session.start_virtual_display.side_effect = RuntimeError('display failed')
            with patch.object(self.runner, 'ViewerSession', return_value=session), \
                 patch.dict(self.runner.os.environ, {}, clear=True), \
                 patch.object(self.runner.subprocess, 'run') as execute:
                with self.assertRaisesRegex(RuntimeError, 'display failed'):
                    self.runner.run_test(SimpleNamespace(executable=Path('/native/test'), evidence=Path(directory)))
                session.__exit__.assert_called_once_with(None, None, None)
                execute.assert_not_called()

    def test_cli_headless_option_uses_existing_isolated_launch_policy(self):
        plan = SimpleNamespace(command=None)
        with patch.object(self.runner.sys, 'argv', ['runner', '--executable', '/native/test', '--evidence', '/tmp/evidence', '--headless']), \
             patch.object(self.runner, 'prepare_headless', return_value=plan) as prepare, \
             patch.object(self.runner, 'use_environment') as environment, \
             patch.object(self.runner, 'run_test', return_value=1) as execute:
            self.assertEqual(self.runner.main(), 1)
            self.assertIn('--headless', prepare.call_args.args[0])
            environment.assert_called_once_with(plan)
            self.assertEqual(execute.call_args.args[0].executable, Path('/native/test'))

    def test_cli_relaunch_propagates_wrapper_failure(self):
        plan = SimpleNamespace(command=['private-wrapper'], environment={'DISPLAY': ':123'})
        with patch.object(self.runner.sys, 'argv', ['runner', '--executable', '/native/test', '--evidence', '/tmp/evidence']), \
             patch.object(self.runner, 'prepare_headless', return_value=plan), \
             patch.object(self.runner.subprocess, 'run', return_value=SimpleNamespace(returncode=2)) as execute, \
             patch.object(self.runner, 'run_test') as test:
            self.assertEqual(self.runner.main(), 2)
            execute.assert_called_once_with(plan.command, env=plan.environment)
            test.assert_not_called()


if __name__ == '__main__': unittest.main()
