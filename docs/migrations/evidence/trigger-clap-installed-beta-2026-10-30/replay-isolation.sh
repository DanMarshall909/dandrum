#!/bin/bash
set -euo pipefail

root=/home/dan/code/.worktrees/dandrum/feat-juce-free-host-adapters
evidence="$root/docs/migrations/evidence/trigger-clap-installed-beta-2026-10-30"
installed=/tmp/dandrum-trigger-7-1-cmake-installed
dynamic_host="$root/build/engine-only-regression/cargo-target/debug/deps/trigger_dynamic-1def899b3f4fbc7c"
gui_host=/tmp/dandrum-trigger-7-1-build/rust-target/release/deps/trigger_gui-ce29576c8b043017
xvfb=/tmp/dandrum-trigger-7-1-xvfb/usr/bin/Xvfb
xauth_file=/tmp/dandrum-trigger-7-1-Xauthority93

verify_inputs() {
  python3 - "$installed" "$evidence/artifact-manifest.json" "$dynamic_host" "$gui_host" "$xvfb" <<'PY'
from pathlib import Path
import hashlib, json, sys
installed, manifest, dynamic, gui, xvfb = map(Path, sys.argv[1:])
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
data = json.loads(manifest.read_text())
files = {p.relative_to(installed).as_posix(): p for p in installed.rglob('*') if p.is_file()}
assert set(files) == {row['path'] for row in data['files']}
assert all(not p.is_symlink() for p in installed.rglob('*'))
assert all(sha(files[row['path']]) == row['sha256'] and files[row['path']].stat().st_size == row['bytes'] for row in data['files'])
expected = {
    dynamic: '980c6093a5a7771cec4ef1a3c45998e2ea797d5418e961ed16128b66378fa1d6',
    gui: 'ee981fa15c1361fd39b1b842111f9415e3a500edd2bcb79f587167c0ed3e9549',
    xvfb: 'cf71d78eab8321f7c568271053af449f966a75422140117ea1f244bf72a7e81e',
}
assert all(sha(path) == value for path, value in expected.items())
print('BOUND installed_files=20 plugin_sha256=' + sha(installed/'lib/clap/DandrumTrigger.clap'))
for path, value in expected.items(): print('BOUND executable_sha256=' + value + ' path=' + str(path))
PY
}

verify_inputs
host_net=$(readlink /proc/self/ns/net)
printf 'OUTER host_net=%s cwd=%s\n' "$host_net" "$(pwd -P)"

bwrap --die-with-parent --ro-bind / / --unshare-net --unshare-pid --proc /proc --dev /dev \
  --tmpfs /home/dan/code --tmpfs /home/dan/.cargo --tmpfs /home/dan/.cache --tmpfs /tmp \
  --ro-bind "$installed" /tmp/installed --ro-bind "$dynamic_host" /tmp/host \
  --ro-bind "$evidence/isolated-headless.sh" /tmp/run \
  --setenv DANDRUM_HOST_NET_NS "$host_net" --chdir /tmp/installed -- /bin/sh /tmp/run \
  > /tmp/dandrum-trigger-7-1-final-headless.log 2>&1
test "$(grep -c 'test result: ok. 1 passed' /tmp/dandrum-trigger-7-1-final-headless.log)" -eq 3
printf 'HEADLESS exit=0 exact_tests=3\n'

xauth -f "$xauth_file" add :93 . "$(mcookie)"
bwrap --die-with-parent --ro-bind / / --unshare-net --unshare-pid --proc /proc --dev /dev \
  --tmpfs /home/dan/code --tmpfs /home/dan/.cargo --tmpfs /home/dan/.cache --tmpfs /tmp \
  --ro-bind "$installed" /tmp/installed --ro-bind "$gui_host" /tmp/host \
  --ro-bind "$xvfb" /tmp/Xvfb --ro-bind "$xauth_file" /tmp/Xauthority \
  --ro-bind "$evidence/isolated-gui.sh" /tmp/run \
  --setenv DANDRUM_HOST_NET_NS "$host_net" --chdir /tmp/installed -- /bin/sh /tmp/run \
  > /tmp/dandrum-trigger-7-1-final-gui.log 2>&1
test "$(grep -c 'test result: ok. 1 passed' /tmp/dandrum-trigger-7-1-final-gui.log)" -eq 1
printf 'GUI exit=0 exact_tests=1\n'
verify_inputs
printf 'REPLAY exit=0 installed_bytes_unchanged=true\n'
