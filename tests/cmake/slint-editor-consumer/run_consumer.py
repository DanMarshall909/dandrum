#!/usr/bin/env python3
"""Build the public Slint API from a source bundle with no sampler present."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cmake', required=True)
    parser.add_argument('--sdk', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--config', default='Release')
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[3]
    fixture = Path(__file__).resolve().parent
    args.evidence.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='dandrum-independent-slint-') as temporary:
        workspace = Path(temporary)
        package = workspace / 'package'
        shutil.copytree(repo / 'ui/slint', package / 'ui/slint')
        shutil.copytree(repo / 'ui/design-system/fonts', package / 'ui/design-system/fonts')
        (args.evidence / 'consumer-sources.json').write_text(json.dumps({
            'sdk': str(args.sdk),
            'catalog': False, 'tests': False, 'file_drop': False,
            'sources': {
                str(file.relative_to(package)): hashlib.sha256(file.read_bytes()).hexdigest()
                for file in sorted(package.rglob('*')) if file.is_file()
            },
        }, indent=2) + '\n')
        build = workspace / 'build'
        commands = [
            [args.cmake, '-S', str(fixture), '-B', str(build),
             f'-DSlint_DIR={args.sdk}', f'-DCMAKE_BUILD_TYPE={args.config}',
             f'-DDANDRUM_SLINT_LIBRARY_SOURCE={package / "ui/slint"}'],
            [args.cmake, '--build', str(build), '--config', args.config, '--parallel', '2'],
        ]
        with (args.evidence / 'consumer-build.log').open('w') as log:
            for command in commands:
                result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=180)
                if result.returncode:
                    print((args.evidence / 'consumer-build.log').read_text(), end='')
                    return result.returncode
            executable = build / ('independent-slint-editor.exe' if os.name == 'nt' else 'independent-slint-editor')
            if not executable.exists():
                executable = build / args.config / executable.name
            result = subprocess.run([str(executable)], stdout=log, stderr=subprocess.STDOUT, timeout=30)
        print((args.evidence / 'consumer-build.log').read_text(), end='')
        return result.returncode


if __name__ == '__main__':
    raise SystemExit(main())
