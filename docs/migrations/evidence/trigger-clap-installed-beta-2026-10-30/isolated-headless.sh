#!/bin/sh
set -eu
test "$(pwd -P)" = /tmp/installed
test -n "${DANDRUM_HOST_NET_NS:-}"
test "$(readlink /proc/self/ns/net)" != "$DANDRUM_HOST_NET_NS"
test ! -e /home/dan/code/.worktrees/dandrum/feat-juce-free-host-adapters/CMakeLists.txt
test ! -e /tmp/dandrum-trigger-7-1-build
test ! -e /home/dan/.cargo/registry
test ! -e /home/dan/.cache/dandrum-trigger-beta
export DANDRUM_TRIGGER_CLAP_PLUGIN_PATH=/tmp/installed/lib/clap/DandrumTrigger.clap
printf 'SOURCE_AND_CACHE_HIDDEN host_net=%s isolated_net=%s cwd=%s\n' \
  "$DANDRUM_HOST_NET_NS" "$(readlink /proc/self/ns/net)" "$(pwd -P)"
/tmp/host optional_product_discovers_stable_trigger_and_retains_fixture_identities --exact --test-threads=1
/tmp/host trigger_initializes_from_only_sibling_resources_and_exposes_frozen_physical_parameters --exact --test-threads=1
/tmp/host fresh_clap_restores_selected_payload_without_a_package_file --exact --test-threads=1
