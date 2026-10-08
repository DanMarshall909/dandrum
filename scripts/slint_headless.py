"""Choose an isolated Linux display without changing Slint verification scope."""
from __future__ import annotations

from contextlib import contextmanager
from dataclasses import dataclass
import os
import shutil
import sys


@dataclass(frozen=True)
class HeadlessLaunch:
    command: list[str] | None
    environment: dict[str, str]


def prepare_headless(argv, environment, *, which=shutil.which, platform=sys.platform, script=None):
    if '--headless' not in argv or '--check-only' in argv:
        return None
    if not platform.startswith('linux'):
        raise RuntimeError('--headless uses Xvfb on Linux; use the normal desktop runner on this platform')
    explicit = environment.get('DANDRUM_XVFB')
    xvfb = which(explicit or 'Xvfb')
    if not xvfb:
        raise RuntimeError('Xvfb was not found; install Xvfb or set DANDRUM_XVFB to its executable. '
                           '--check-only compiles without a display; --headless still runs real input checks.')
    env = dict(environment)
    # Never let a requested headless run attach to the user's desktop.
    env.pop('DISPLAY', None)
    env.pop('WAYLAND_DISPLAY', None)
    env.update(DANDRUM_SLINT_HEADLESS='1', PYTHONDONTWRITEBYTECODE='1')
    wrapper = which('xvfb-run')
    if wrapper and which('xauth') and not explicit:
        command = [wrapper, '-a', '-s', '-screen 0 1920x1200x24 -nolisten tcp',
                   sys.executable, str(script or sys.argv[0]),
                   *[arg for arg in argv if arg != '--headless']]
        return HeadlessLaunch(command, env)
    # ViewerSession already creates an authenticated display per live window and
    # terminates its own server on success, assertion failure or launch failure.
    env['DANDRUM_XVFB'] = xvfb
    return HeadlessLaunch(None, env)


@contextmanager
def use_environment(plan):
    if plan is None:
        yield
        return
    previous = dict(os.environ)
    os.environ.clear()
    os.environ.update(plan.environment)
    try:
        yield
    finally:
        os.environ.clear()
        os.environ.update(previous)
