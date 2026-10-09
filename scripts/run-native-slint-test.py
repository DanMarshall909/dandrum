#!/usr/bin/env python3
"""Run a compiled Slint renderer test on a desktop or private virtual display."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / 'scripts'))
sys.path.insert(0, str(REPO / 'tests/slint'))
from mcp_client import ViewerSession
from slint_headless import prepare_headless, use_environment


def run_test(args):
    args.evidence.mkdir(parents=True, exist_ok=True)
    display = ViewerSession('', args.executable, args.evidence)
    display.temporary = tempfile.TemporaryDirectory(prefix='dandrum-native-test-')
    display.directory = Path(display.temporary.name)
    display.env = dict(os.environ)
    try:
        if not (display.env.get('DISPLAY') or display.env.get('WAYLAND_DISPLAY')):
            display.start_virtual_display()
        with (args.evidence / 'native-test.log').open('w') as output:
            result = subprocess.run([str(args.executable), str(args.evidence)], env=display.env,
                                    stdout=output, stderr=subprocess.STDOUT, timeout=90)
        print((args.evidence / 'native-test.log').read_text(), end='')
        return result.returncode
    finally:
        display.__exit__(None, None, None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--headless', action='store_true')
    args = parser.parse_args()
    args.executable = args.executable.resolve()
    plan = prepare_headless(sys.argv[1:], os.environ)
    if plan and plan.command:
        return subprocess.run(plan.command, env=plan.environment).returncode
    with use_environment(plan):
        return run_test(args)


if __name__ == '__main__': raise SystemExit(main())
