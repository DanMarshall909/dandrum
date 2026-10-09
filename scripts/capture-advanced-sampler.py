#!/usr/bin/env python3
"""Capture the compiled native sampler, preserving every design state and finish."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import zlib

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / 'scripts'))
sys.path.insert(0, str(REPO / 'tests/slint'))
from mcp_client import ViewerSession
from slint_headless import prepare_headless, use_environment


def capture_cases(all_sizes=False):
    states = ['empty', 'loaded', 'sample', 'loop', 'slices', 'transients', 'mapping', 'vel', 'rr',
              'voice', 'mod', 'macro', 'routing', 'fx', 'missing', 'unsupported', 'loading',
              'running', 'failed', 'min', 'default', 'exp', 'drop', 'perf', 'pads', 'recon',
              'diag', 'overview', 'freeze', 'macros']
    surfaces = ['aluminium', 'brushed', 'ember', 'graphite', 'midnight', 'forest', 'camo', 'paper']
    rows = [dict(kind='state', state=s, size='min' if s == 'min' else 'expanded' if s == 'exp' else 'default',
                 theme='aluminium', finish='soft', name=s) for s in states]
    rows.extend(dict(kind='theme', state='sample', size='default', theme=s, finish=f, name=f'{s}-{f}')
                for s in surfaces for f in ['soft', 'flat'])
    if all_sizes:
        pages = ['empty', 'pads', 'macros', 'sample', 'slices', 'mapping', 'vel',
                 'voice', 'mod', 'routing', 'fx', 'overview', 'diag']
        rows.extend(dict(kind='page-size', state=page, size=size, theme='aluminium', finish='soft',
                         name=f'page-{page}-{size}')
                    for page in pages for size in ['min', 'default', 'expanded'])
    return rows


def snapshot_to_png(source, target):
    raw = source.read_bytes()
    header, separator, pixels = raw.partition(b'ENDHDR\n')
    if not separator or not header.startswith(b'P7\n'): raise ValueError('Invalid native snapshot header')
    fields = dict(line.split(b' ', 1) for line in header.splitlines()[1:] if b' ' in line)
    width, height = int(fields[b'WIDTH']), int(fields[b'HEIGHT'])
    if width <= 0 or height <= 0 or fields[b'DEPTH'] != b'4' or fields[b'MAXVAL'] != b'255' or len(pixels) != width * height * 4:
        raise ValueError('Incomplete or unsupported native snapshot')
    def chunk(kind, payload):
        return struct.pack('>I', len(payload)) + kind + payload + struct.pack('>I', zlib.crc32(kind + payload))
    scanlines = b''.join(b'\0' + pixels[row * width * 4:(row + 1) * width * 4] for row in range(height))
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
    png += chunk(b'IDAT', zlib.compress(scanlines)) + chunk(b'IEND', b'')
    target.write_bytes(png)
    return width, height


def capture(args):
    args.evidence.mkdir(parents=True, exist_ok=True)
    # Reuse the maintained authenticated Xvfb lifecycle. No viewer or MCP server
    # is launched: all images come from the real C++ application snapshot API.
    display = ViewerSession('', REPO / 'ui/advanced-sampler/AppWindow.slint', args.evidence)
    display.temporary = tempfile.TemporaryDirectory(prefix='dandrum-native-capture-')
    display.directory = Path(display.temporary.name)
    display.env = dict(os.environ)
    results = []
    try:
        if not (display.env.get('DISPLAY') or display.env.get('WAYLAND_DISPLAY')):
            display.start_virtual_display()
        cases = capture_cases(args.all_sizes)
        if args.state: cases = [r for r in cases if r['kind'] != 'theme' and r['state'] in args.state]
        for row in cases:
            snapshot = args.evidence / (row['name'] + '.pam')
            png = snapshot.with_suffix('.png')
            command = [str(args.app), '--state', row['state'], '--size', row['size'],
                       '--theme', row['theme'], '--finish', row['finish'], '--screenshot', str(snapshot)]
            result = subprocess.run(command, env=display.env, capture_output=True, text=True, timeout=30)
            (args.evidence / (row['name'] + '.log')).write_text(result.stdout + result.stderr)
            if result.returncode: raise RuntimeError(f"Native {row['name']} capture failed: {result.stderr}")
            width, height = snapshot_to_png(snapshot, png)
            results.append(dict(row, width=width, height=height, file=png.name,
                                sha256=hashlib.sha256(png.read_bytes()).hexdigest()))
            print(f"Captured native {row['name']}: {width}×{height}", flush=True)
    finally:
        display.__exit__(None, None, None)
    report = dict(renderer=display.env.get('SLINT_BACKEND'), headless=display.env.get('DANDRUM_SLINT_HEADLESS') == '1',
                  executable=str(args.app), executable_sha256=hashlib.sha256(args.app.read_bytes()).hexdigest(), captures=results)
    (args.evidence / 'native-captures.json').write_text(json.dumps(report, indent=2) + '\n')
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, default=REPO / 'build/advanced-sampler-evidence/native')
    parser.add_argument('--headless', action='store_true')
    parser.add_argument('--all-sizes', action='store_true', help='Also capture all 13 workspaces at each editor size')
    parser.add_argument('--state', action='append', choices=[r['state'] for r in capture_cases() if r['kind'] == 'state'])
    args = parser.parse_args()
    args.app = args.app.resolve()
    plan = prepare_headless(sys.argv[1:], os.environ)
    if plan and plan.command: return subprocess.run(plan.command, env=plan.environment).returncode
    with use_environment(plan): return capture(args)


if __name__ == '__main__': raise SystemExit(main())
