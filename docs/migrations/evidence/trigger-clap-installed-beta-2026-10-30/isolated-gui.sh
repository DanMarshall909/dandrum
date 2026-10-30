#!/bin/sh
set -eu
test ! -e /home/dan/code/.worktrees/dandrum/feat-juce-free-host-adapters/CMakeLists.txt
test ! -e /tmp/dandrum-trigger-7-1-build
test ! -e /home/dan/.cargo/registry
test ! -e /home/dan/.cache/dandrum-trigger-beta
export DISPLAY=localhost:93.0
export XAUTHORITY=/tmp/Xauthority
export GDK_BACKEND=x11
export LIBGL_ALWAYS_SOFTWARE=1
export __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json
cp -a /tmp/installed /tmp/private
export DANDRUM_TRIGGER_GUI_PLUGIN_PATH=/tmp/private/lib/clap/DandrumTrigger.clap
/tmp/Xvfb :93 -screen 0 1280x800x24 -nolisten unix -listen tcp -auth /tmp/Xauthority > /tmp/xvfb.log 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true; wait "$server" 2>/dev/null || true' EXIT INT TERM
n=0
while ! /usr/bin/xdpyinfo >/dev/null 2>&1; do
  n=$((n+1))
  test "$n" -lt 40 || { cat /tmp/xvfb.log; exit 1; }
  sleep .1
done
printf 'SOURCE_AND_CACHE_HIDDEN_NETWORK_NAMESPACE_ACTIVE\n'
/tmp/host copied_native_editor_lifecycle_initializes_the_real_react_session_and_preserves_audio --ignored --exact --test-threads=1
