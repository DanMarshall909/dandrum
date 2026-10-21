"""Prove the actual engine build with the JUCE/browser source trees absent."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CMAKE = shutil.which("cmake")
CTEST = shutil.which("ctest")


@unittest.skipUnless(sys.platform.startswith("linux"), "existing Rust static-library link recipe is Linux-specific")
class EngineBuildFixture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Keep the task-local Cargo/build cache reusable across regression runs.
        cls.work = ROOT / "build" / "engine-only-regression"
        cls.work.mkdir(parents=True, exist_ok=True)
        # Fresh snapshots cannot retain removed source files or discovery caches.
        cls.fixture_root = Path(tempfile.mkdtemp(prefix="fixture-", dir=cls.work))
        cls.source = cls.fixture_root / "source"
        cls.source.mkdir()
        cls.cargo_target = cls.work / "cargo-target"
        cls.cargo_target.mkdir(exist_ok=True)
        shutil.copy2(ROOT / "CMakeLists.txt", cls.source / "CMakeLists.txt")
        for name in ("src", "tests", "examples", "schema", "scripts", "spikes"):
            shutil.copytree(ROOT / name, cls.source / name, ignore=shutil.ignore_patterns("target", "build", "__pycache__"))
        assert not (cls.source / "third_party").exists()
        assert not (cls.source / "web").exists()
        assert not (cls.source / "ui").exists()

    def command(self, label, args):
        result = subprocess.run(args, capture_output=True, text=True,
                                env=dict(os.environ, CARGO_NET_OFFLINE="true"))
        output = result.stdout + result.stderr
        (self.work / (label + ".log")).write_text(output)
        return result.returncode, output

    def configure(self, label, *options):
        # DefaultPatch searches upward from CTest; keep it inside this snapshot.
        build = self.source / "build" / label
        build.mkdir(parents=True)
        if "-DDANDRUM_ENGINE_ONLY=ON" in options:
            # All fixtures belong to this task; CTest serializes this guard.
            (build / "rust-target").symlink_to(self.cargo_target, target_is_directory=True)
        code, output = self.command(label, [CMAKE, "-S", str(self.source), "-B", str(build),
                                          "-DCMAKE_BUILD_TYPE=Release", *options])
        return build, code, output


class EngineOnlyBuildTest(EngineBuildFixture):
    def test_engine_build_without_host_dependencies(self):
        build, code, output = self.configure("engine", "-DDANDRUM_ENGINE_ONLY=ON")
        self.assertEqual(code, 0, output)
        cache = (build / "CMakeCache.txt").read_text()
        self.assertNotIn("DANDRUM_NODE_EXECUTABLE:", cache)
        code, output = self.command("engine-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        for target in ("juceaide", "dandrum-plugin", "dandrum-sampler-plugin",
                       "dandrum-drum-machine-demo", "dandrum-native-knob-fonts"):
            self.assertNotRegex(output, r"(?m)(?:^|\s)" + target + r"(?:\s|$)")
        # Build every independent native target, so accidental adapter links fail.
        code, output = self.command("engine-build", [CMAKE, "--build", str(build), "--parallel", "2"])
        self.assertEqual(code, 0, output)
        code, output = self.command("engine-inventory", [CTEST, "--test-dir", str(build),
                                                       "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        names = {test["name"] for test in json.loads(output)["tests"]}
        self.assertNotIn("clap-dynamic-host", names)
        self.assertTrue({"rust-engine-tests", "instrument-session-tests", "cxx-rust-engine-ffi-smoke",
                         "cxx-rust-engine-sample-accurate-midi-smoke", "sound-lab-controller",
                         "cxx-plugin-waveform-service", "cxx-plugin-meter-capture",
                         "filter-spike-engine"}.issubset(names), names)
        # Exercise real Rust audio and session ownership with host sources absent.
        code, output = self.command("engine-runtime", [CTEST, "--test-dir", str(build),
            "--output-on-failure", "-R", "^(cxx-rust-engine-(ffi-smoke|sample-accurate-midi-smoke)|instrument-session-tests)$"])
        self.assertEqual(code, 0, output)
        self.assertIn("100% tests passed", output)

    def test_default_and_disabled_engine_mode_require_comparison_host(self):
        for label, options in (("default-host", []), ("explicit-host", ["-DDANDRUM_ENGINE_ONLY=OFF"])):
            with self.subTest(mode=label):
                _, code, output = self.configure(label, "-DDANDRUM_NATIVE_ONLY=ON", *options)
                self.assertNotEqual(code, 0, output)
                self.assertIn("third_party/JUCE", output)
                self.assertNotIn("Node.js is required", output)

    def test_conflicting_product_options_fail_before_dependency_discovery(self):
        options = ("DANDRUM_NATIVE_ONLY", "DANDRUM_TRIGGER_REACT",
                   "DANDRUM_BUILD_FILTER_UI_SPIKES", "DANDRUM_BUILD_TRIGGER_SLINT",
                   "DANDRUM_SLINT_LIBRARY_ONLY", "DANDRUM_ADVANCED_SAMPLER_ONLY")
        for option in options:
            with self.subTest(option=option):
                _, code, output = self.configure("conflict-" + option,
                    "-DDANDRUM_ENGINE_ONLY=ON", "-D" + option + "=ON")
                self.assertNotEqual(code, 0, output)
                self.assertIn("DANDRUM_ENGINE_ONLY cannot be combined with " + option, " ".join(output.split()))
                self.assertNotIn("third_party/JUCE", output)
                self.assertNotIn("React rendering test dependencies", output)


class ClapProofBuildTest(EngineBuildFixture):
    def test_headless_clap_development_dependencies_exclude_native_editor(self):
        # Dev dependencies are compiled by Cargo tests even if a test source is
        # feature-gated; inspect the complete development graph for both modes.
        for mode, features in (("default", []), ("trigger", ["--features", "trigger"])):
            with self.subTest(mode=mode):
                code, output = self.command(
                    "clap-" + mode + "-development-dependencies",
                    ["cargo", "tree", "--offline", "--locked", "--manifest-path",
                     str(self.source / "src/clap-adapter/Cargo.toml"),
                     "--edges", "normal,build,dev", "--prefix", "none", *features])
                self.assertEqual(code, 0, output)
                names = {line.split()[0].lower()
                         for line in output.splitlines() if line.strip()}
                self.assertIn("dandrum_clap_adapter", names)
                native = sorted(name for name in names
                                if any(part in name for part in
                                       ("wry", "webkit", "gtk", "gdk", "juce")))
                self.assertEqual(native, [],
                                 mode + " headless development graph includes native editor dependencies")

    def test_clap_selection_requires_independent_engine_mode(self):
        _, code, output = self.configure("clap-needs-engine-mode", "-DDANDRUM_CLAP_PROOF=ON")
        self.assertNotEqual(code, 0, output)
        self.assertIn("DANDRUM_CLAP_PROOF requires DANDRUM_ENGINE_ONLY", output)
        self.assertNotIn("Node.js is required", output)
        self.assertNotIn("third_party/JUCE", output)

    def test_clap_rebuilds_when_embedded_engine_resources_change(self):
        build, code, output = self.configure("clap-incremental", "-G", "Unix Makefiles",
            "-DDANDRUM_ENGINE_ONLY=ON", "-DDANDRUM_CLAP_PROOF=ON")
        self.assertEqual(code, 0, output)
        # Inspect the real build tool's incremental plan without recompiling DSP
        # for every input. The full artifact/load test below proves execution.
        artifact = build / "clap" / "DandrumHeadlessProof.clap"
        artifact.parent.mkdir()
        artifact.touch()
        command = [CMAKE, "--build", str(build), "--target", "dandrum-clap-proof", "--", "--dry-run"]
        code, output = self.command("clap-current-plan", command)
        self.assertEqual(code, 0, output)
        self.assertNotIn("cargo build", output)
        for relative in ("schema/patch.schema.yaml", "src/rust-engine/module-library/seed-1.0.0.zip"):
            with self.subTest(resource=relative):
                resource = self.source / relative
                previous = resource.stat()
                newer = artifact.stat().st_mtime_ns + 2_000_000_000
                os.utime(resource, ns=(previous.st_atime_ns, newer))
                try:
                    code, output = self.command("clap-resource-plan-" + resource.name, command)
                    self.assertEqual(code, 0, output)
                    self.assertIn("cargo build", output, relative + " must rebuild the packaged plugin")
                finally:
                    os.utime(resource, ns=(previous.st_atime_ns, previous.st_mtime_ns))

    def test_exported_clap_build_and_dynamic_tests_without_host_sources(self):
        build, code, output = self.configure("clap", "-DDANDRUM_ENGINE_ONLY=ON",
                                             "-DDANDRUM_CLAP_PROOF=ON")
        self.assertEqual(code, 0, output)
        cache = (build / "CMakeCache.txt").read_text()
        self.assertNotIn("DANDRUM_NODE_EXECUTABLE:", cache)
        code, output = self.command("clap-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        self.assertRegex(output, r"(?m)(?:^|\s)dandrum-clap-proof(?:\s|$)")
        self.assertNotRegex(output, r"(?m)(?:^|\s)(?:juceaide|dandrum-plugin)(?:\s|$)")
        code, output = self.command("clap-build", [CMAKE, "--build", str(build),
                                                   "--target", "dandrum-clap-proof", "--parallel", "2"])
        self.assertEqual(code, 0, output)
        self.assertTrue((build / "clap" / "DandrumHeadlessProof.clap").is_file())
        code, output = self.command("clap-runtime", [CTEST, "--test-dir", str(build),
                                                     "--output-on-failure", "-R", "^clap-dynamic-host$"])
        self.assertEqual(code, 0, output)
        self.assertIn("100% tests passed", output)

    def test_trigger_product_build_and_host_check_are_explicit_opt_in(self):
        disabled, code, output = self.configure(
            "trigger-product-disabled", "-DDANDRUM_ENGINE_ONLY=ON",
            "-DDANDRUM_CLAP_PROOF=ON")
        self.assertEqual(code, 0, output)
        code, output = self.command("trigger-product-disabled-targets",
                                   [CMAKE, "--build", str(disabled), "--target", "help"])
        self.assertEqual(code, 0, output)
        self.assertNotRegex(output, r"(?m)(?:^|\s)dandrum-trigger-clap-product(?:\s|$)")
        code, output = self.command("trigger-product-disabled-tests",
                                   [CTEST, "--test-dir", str(disabled), "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        self.assertNotIn("trigger-clap-dynamic-host",
                         {test["name"] for test in json.loads(output)["tests"]})
        bundle = self.fixture_root / "trigger-product-input"
        bundle.mkdir()
        for name in ("index.html", "app.js", "app.css"):
            (bundle / name).write_text("build-interface fixture")
        enabled, code, output = self.configure(
            "trigger-product-enabled", "-DDANDRUM_ENGINE_ONLY=ON",
            "-DDANDRUM_CLAP_PROOF=ON", "-DDANDRUM_TRIGGER_CLAP_EDITOR=ON",
            "-DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=" + str(bundle))
        self.assertEqual(code, 0, output)
        code, output = self.command("trigger-product-enabled-targets",
                                   [CMAKE, "--build", str(enabled), "--target", "help"])
        self.assertEqual(code, 0, output)
        self.assertRegex(output, r"(?m)(?:^|\s)dandrum-trigger-clap-product(?:\s|$)")
        code, output = self.command("trigger-product-enabled-tests",
                                   [CTEST, "--test-dir", str(enabled), "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        tests = {test["name"]: test for test in json.loads(output)["tests"]}
        command = tests["trigger-clap-dynamic-host"]["command"]
        self.assertIn("DANDRUM_TRIGGER_CLAP_PLUGIN_PATH="
                      + str(enabled / "clap/DandrumTrigger.clap"), command)
        self.assertIn("trigger", command)
        self.assertIn("trigger_dynamic", command)
        self.assertIn("DANDRUM_CLAP_PLUGIN_PATH="
                      + str(enabled / "clap/DandrumHeadlessProof.clap"),
                      tests["clap-dynamic-host"]["command"])

    def test_native_editor_lifetime_checks_have_prepared_bounded_exact_tests(self):
        bundle = self.fixture_root / "native-registration-input"
        bundle.mkdir()
        for name in ("index.html", "app.js", "app.css"):
            (bundle / name).write_text("native registration fixture")

        def inventory(label, build):
            code, output = self.command(label, [CTEST, "--test-dir", str(build),
                                                "--show-only=json-v1"])
            self.assertEqual(code, 0, output)
            return {test["name"]: test for test in json.loads(output)["tests"]}

        options = ("-DDANDRUM_ENGINE_ONLY=ON", "-DDANDRUM_CLAP_PROOF=ON",
                   "-DDANDRUM_TRIGGER_CLAP_EDITOR=ON",
                   "-DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=" + str(bundle))
        disabled, code, output = self.configure("native-owner-disabled", *options)
        self.assertEqual(code, 0, output)
        disabled_tests = inventory("native-owner-disabled-inventory", disabled)
        for name in ("trigger-editor-native-prepare", "trigger-editor-native-x11",
                     "trigger-editor-native-handler-lifetime"):
            self.assertNotIn(name, disabled_tests)

        enabled, code, output = self.configure(
            "native-owner-enabled", *options, "-DDANDRUM_TRIGGER_EDITOR_NATIVE_X11=ON")
        self.assertEqual(code, 0, output)
        tests = inventory("native-owner-enabled-inventory", enabled)
        self.assertIn("trigger-editor-native-handler-lifetime", tests,
                      "native destruction ownership must run in the maintained native lane")
        prepare = tests["trigger-editor-native-prepare"]
        preparation = {prop["name"]: prop["value"] for prop in prepare["properties"]}
        for argument in ("--release", "--locked", "native-x11", "--lib", "--no-run"):
            self.assertIn(argument, prepare["command"])
        self.assertNotIn("--ignored", prepare["command"])
        self.assertIn("trigger-editor-native-compiled", preparation["FIXTURES_SETUP"])
        self.assertEqual(preparation["TIMEOUT"], 1200.0)
        self.assertTrue(preparation["RUN_SERIAL"])

        cases = {
            "trigger-editor-native-x11":
                "native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent",
            "trigger-editor-native-handler-lifetime":
                "native_x11::tests::destroy_releases_application_handlers_while_framework_context_remains",
        }
        for name, case in cases.items():
            with self.subTest(native_case=name):
                test = tests[name]
                for argument in ("--release", "--locked", "native-x11", "--lib", case,
                                 "--ignored", "--exact", "--test-threads=1"):
                    self.assertIn(argument, test["command"])
                self.assertNotIn("--no-run", test["command"])
                properties = {prop["name"]: prop["value"] for prop in test["properties"]}
                self.assertIn("trigger-editor-native-compiled", properties["FIXTURES_REQUIRED"])
                self.assertEqual(properties["TIMEOUT"], 60.0)
                self.assertTrue(properties["RUN_SERIAL"])
                self.assertIn("native-x11", properties["LABELS"])
                self.assertIn("running 0 tests", properties["FAIL_REGULAR_EXPRESSION"],
                              "an exact filter matching no test must fail the native gate")

    def test_private_clap_native_fixture_uses_reported_artifacts_and_exact_processes(self):
        bundle = self.fixture_root / "clap-native-registration-input"
        bundle.mkdir()
        for name in ("index.html", "app.js", "app.css"):
            (bundle / name).write_text("native registration fixture")
        options = ("-DDANDRUM_ENGINE_ONLY=ON", "-DDANDRUM_CLAP_PROOF=ON",
                   "-DDANDRUM_TRIGGER_CLAP_EDITOR=ON",
                   "-DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=" + str(bundle))

        def inventory(label, build):
            code, output = self.command(label, [CTEST, "--test-dir", str(build),
                                                "--show-only=json-v1"])
            self.assertEqual(code, 0, output)
            return {test["name"]: test for test in json.loads(output)["tests"]}

        disabled, code, output = self.configure("clap-native-disabled", *options)
        self.assertEqual(code, 0, output)
        disabled_tests = inventory("clap-native-disabled-inventory", disabled)
        self.assertNotIn("trigger-clap-native-prepare", disabled_tests)
        self.assertNotIn("trigger-clap-native-parameters-prepare", disabled_tests)
        self.assertFalse(any(name.startswith("trigger-clap-native-parameter-")
                             for name in disabled_tests))
        enabled, code, output = self.configure(
            "clap-native-enabled", *options, "-DDANDRUM_TRIGGER_EDITOR_NATIVE_X11=ON")
        self.assertEqual(code, 0, output)
        tests = inventory("clap-native-enabled-inventory", enabled)
        self.assertIn("trigger-clap-native-prepare", tests,
                      "native CLAP checks need a Cargo-reported artifact fixture")
        prepare = tests["trigger-clap-native-prepare"]
        fixture = enabled / "trigger-clap-native-fixture"
        for argument in (str(self.source / "scripts/prepare-trigger-clap-native.py"),
                         "--cargo", "--manifest", "--target-dir", "--resource-dir",
                         str(enabled / "clap/DandrumTrigger.resources"),
                         "--output-dir", str(fixture)):
            self.assertIn(argument, prepare["command"])
        properties = {prop["name"]: prop["value"] for prop in prepare["properties"]}
        self.assertIn("trigger-clap-native-compiled", properties["FIXTURES_SETUP"])
        self.assertEqual(properties["TIMEOUT"], 1200.0)
        self.assertTrue(properties["RUN_SERIAL"])
        cases = {
            "trigger-clap-native-extension":
                "copied_trigger_editor_exposes_the_actual_clap_gui_extension",
            "trigger-clap-native-lifecycle":
                "copied_native_editor_lifecycle_initializes_the_real_react_session_and_preserves_audio",
            "trigger-clap-native-configuration":
                "copied_editor_scan_and_configuration_only_do_not_make_code_resident",
            "trigger-clap-native-loader-identity":
                "copied_editor_rejects_wrong_loaded_entry_identity_before_gtk_without_state_or_audio_loss",
            "trigger-clap-native-failed-parent":
                "copied_editor_failed_native_parent_attempt_retains_only_code_and_preserves_audio",
        }
        for name, case in cases.items():
            with self.subTest(native_case=name):
                test = tests[name]
                for argument in (str(fixture / "trigger-gui-test"), "GDK_BACKEND=x11",
                                 "DANDRUM_TRIGGER_GUI_PLUGIN_PATH=" + str(fixture / "DandrumTrigger.clap"),
                                 case, "--ignored", "--exact", "--test-threads=1"):
                    self.assertIn(argument, test["command"])
                self.assertNotIn("--manifest-path", test["command"])
                props = {prop["name"]: prop["value"] for prop in test["properties"]}
                self.assertIn("trigger-clap-native-compiled", props["FIXTURES_REQUIRED"])
                self.assertEqual(props["TIMEOUT"], 60.0)
                self.assertTrue(props["RUN_SERIAL"])
                self.assertIn("running 0 tests", props["FAIL_REGULAR_EXPRESSION"])
        # The separate developer fixture must not change the maintained product.
        product_rule = (enabled / "CMakeFiles/dandrum-trigger-clap-product.dir/build.make").read_text()
        self.assertIn("--features trigger --lib", product_rule)
        self.assertNotIn("--features trigger-editor", product_rule)
        self.assertIn("trigger-clap-native-parameters-prepare", tests,
                      "parameter IPC needs its separately reported native test executable")
        parameter_fixture = enabled / "trigger-clap-parameters-fixture"
        prepare = tests["trigger-clap-native-parameters-prepare"]
        for argument in ("--test-target", "trigger_parameters", "--output-dir", str(parameter_fixture)):
            self.assertIn(argument, prepare["command"])
        props = {prop["name"]: prop["value"] for prop in prepare["properties"]}
        self.assertIn("trigger-clap-parameters-compiled", props["FIXTURES_SETUP"])
        self.assertTrue(props["RUN_SERIAL"])
        self.assertEqual(props["TIMEOUT"], 1200.0)
        parameter_cases = {
            "trigger-clap-native-parameter-state":
                "copied_native_parameter_state_is_an_authenticated_numeric_dto",
            "trigger-clap-native-parameter-flush":
                "copied_native_parameter_flush_delivers_host_accepted_gestures",
            "trigger-clap-native-parameter-ownership":
                "copied_native_automation_survives_rejected_output_activation_failures_and_audio_return",
            "trigger-clap-native-parameter-timer-required":
                "copied_native_gui_requires_host_timer",
            "trigger-clap-native-parameter-timer-refused":
                "copied_native_gui_rejects_refused_host_timer",
            "trigger-clap-native-parameter-timer-delivery":
                "copied_native_host_timer_delivers_a_later_rendered_parameter_edit",
            "trigger-clap-native-parameter-timer-rollback":
                "copied_native_timer_rollback_retains_disabled_cleanup_and_retries",
            "trigger-clap-native-parameter-source-credits":
                "copied_native_view_independent_source_collects_accepted_automation_credits",
            "trigger-clap-native-parameter-feedback-undo":
                "copied_native_host_feedback_updates_rendered_control_without_polling_or_undo_pollution",
            "trigger-clap-native-parameter-callback-audit":
                "copied_native_parameter_callbacks_allocate_and_destroy_nothing_while_gui_services",
            "trigger-clap-native-parameter-invalid-stopped-flush":
                "copied_native_invalid_stopped_flush_preserves_uncollected_automation_ownership",
            "trigger-clap-native-parameter-structural-session":
                "structural_native::copied_native_mapping_survives_lost_reply_and_view_loss",
            "trigger-clap-native-parameter-note-hold":
                "note_native::copied_native_gated_sample_outlasts_short_fixture_and_releases",
            "trigger-clap-native-parameter-note-heartbeat":
                "note_native::copied_native_heartbeat_renews_held_note_past_original_deadline",
            "trigger-clap-native-parameter-note-expiry":
                "note_native::copied_native_live_attachment_expires_renewed_note_and_preserves_host_voice",
        }
        for name, case in parameter_cases.items():
            with self.subTest(parameter_case=name):
                self.assertIn(name, tests, "parameter behavior needs an exact native process")
                test = tests[name]
                for argument in (str(parameter_fixture / "trigger-parameters-test"),
                                 "DANDRUM_TRIGGER_GUI_PLUGIN_PATH=" + str(parameter_fixture / "DandrumTrigger.clap"),
                                 case, "--ignored", "--exact", "--test-threads=1"):
                    self.assertIn(argument, test["command"])
                props = {prop["name"]: prop["value"] for prop in test["properties"]}
                self.assertIn("trigger-clap-parameters-compiled", props["FIXTURES_REQUIRED"])
                self.assertTrue(props["RUN_SERIAL"])
                self.assertEqual(props["TIMEOUT"], 60.0)
                self.assertIn("running 0 tests", props["FAIL_REGULAR_EXPRESSION"])

    def test_trigger_editor_scaffold_stages_copied_resources_without_host_sources(self):
        editor_input = self.fixture_root / "editor-input"
        editor_input.mkdir()
        editor_files = {
            "index.html": b"<!doctype html><div id='root'></div>",
            "app.js": b"window.__triggerEditorFixture = true;",
            "app.css": b":root { color-scheme: dark; }",
            "fonts/fixture.woff2": b"first-font-revision",
        }
        for relative, contents in editor_files.items():
            destination = editor_input / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(contents)

        instrument_source = self.source / "examples/patches/advanced-drum-kit.yaml"
        sample_source = self.source / "examples/patches/assets/advanced-drums.wav"
        expected = {
            "editor/index.html": editor_files["index.html"],
            "editor/app.js": editor_files["app.js"],
            "editor/app.css": editor_files["app.css"],
            "editor/fonts/fixture.woff2": editor_files["fonts/fixture.woff2"],
            "instrument/advanced-drum-kit.yaml": instrument_source.read_bytes(),
            "instrument/assets/advanced-drums.wav": sample_source.read_bytes(),
        }

        build, code, output = self.configure(
            "trigger-editor", "-DDANDRUM_ENGINE_ONLY=ON", "-DDANDRUM_CLAP_PROOF=ON",
            "-DDANDRUM_TRIGGER_CLAP_EDITOR=ON",
            "-DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=" + str(editor_input))
        self.assertEqual(code, 0, output)
        cache = (build / "CMakeCache.txt").read_text()
        self.assertNotIn("DANDRUM_NODE_EXECUTABLE:", cache)

        code, output = self.command(
            "trigger-editor-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        self.assertRegex(output, r"(?m)(?:^|\s)dandrum-trigger-clap-editor(?:\s|$)")
        self.assertRegex(output, r"(?m)(?:^|\s)dandrum-trigger-host-rust(?:\s|$)")
        self.assertRegex(output, r"(?m)(?:^|\s)dandrum-trigger-editor-rust(?:\s|$)")
        self.assertNotRegex(output, r"(?m)(?:^|\s)(?:juceaide|dandrum-plugin)(?:\s|$)")

        code, output = self.command(
            "trigger-host-test-inventory", [CTEST, "--test-dir", str(build),
                "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        names = {test["name"] for test in json.loads(output)["tests"]}
        self.assertIn("trigger-host-tests", names)
        self.assertIn("trigger-editor-tests", names)
        self.assertIn("trigger-editor-packaged-assets", names)

        # This tiny bundle is inventory/build-only. Never execute the native
        # production React/font calibration with its placeholder resources.
        native_build, code, output = self.configure(
            "trigger-native-inventory", "-DDANDRUM_ENGINE_ONLY=ON", "-DDANDRUM_CLAP_PROOF=ON",
            "-DDANDRUM_TRIGGER_CLAP_EDITOR=ON", "-DDANDRUM_TRIGGER_EDITOR_NATIVE_X11=ON",
            "-DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=" + str(editor_input))
        self.assertEqual(code, 0, output)
        code, output = self.command(
            "trigger-native-test-inventory", [CTEST, "--test-dir", str(native_build),
                                              "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        native_test = next(test for test in json.loads(output)["tests"]
                           if test["name"] == "trigger-editor-native-x11")
        self.assertIn(
            "DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR="
            + str(native_build / "clap/DandrumTrigger.resources/editor"),
            native_test["command"],
            "native production calibration requires the exact staged bundle argument")

        code, output = self.command(
            "trigger-editor-build", [CMAKE, "--build", str(build), "--target",
                                     "dandrum-trigger-clap-editor", "--parallel", "2"])
        self.assertEqual(code, 0, output)
        default_artifact = build / "clap/DandrumHeadlessProof.clap"
        artifact = build / "clap/DandrumTrigger.clap"
        resources = build / "clap/DandrumTrigger.resources"
        self.assertTrue(default_artifact.is_file())
        self.assertTrue(artifact.is_file())
        default_binary = default_artifact.read_bytes()
        trigger_binary = artifact.read_bytes()
        for relative, contents in expected.items():
            copied = resources / relative
            self.assertTrue(copied.is_file(), relative)
            self.assertFalse(copied.is_symlink(), relative)
            self.assertEqual(copied.read_bytes(), contents, relative)

        updated_font = b"second-font-revision"
        (editor_input / "fonts/fixture.woff2").write_bytes(updated_font)
        code, output = self.command(
            "trigger-editor-resource-rebuild", [CMAKE, "--build", str(build), "--target",
                                                 "dandrum-trigger-clap-editor", "--parallel", "2"])
        self.assertEqual(code, 0, output)
        self.assertEqual((resources / "editor/fonts/fixture.woff2").read_bytes(), updated_font)
        expected["editor/fonts/fixture.woff2"] = updated_font
        self.assertEqual(default_artifact.read_bytes(), default_binary)
        self.assertEqual(artifact.read_bytes(), trigger_binary)

        code, output = self.command(
            "trigger-host-dependencies", ["cargo", "tree", "--offline", "--locked",
                "--manifest-path", str(self.source / "src/trigger-host/Cargo.toml"),
                "--edges", "normal", "--prefix", "none"])
        self.assertEqual(code, 0, output)
        dependency_names = {line.split()[0].lower()
                            for line in output.splitlines() if line.strip()}
        for forbidden in ("clack", "wry", "webkit", "gtk", "juce"):
            self.assertFalse(any(forbidden in name for name in dependency_names),
                             (forbidden, dependency_names))

        code, output = self.command(
            "trigger-editor-fixture-runtime", [CTEST, "--test-dir", str(build),
                "--output-on-failure", "-R",
                "^(clap-dynamic-host|trigger-clap-dynamic-host|trigger-host-tests|trigger-editor-tests)$"])
        self.assertEqual(code, 0, output)
        self.assertIn("100% tests passed", output)

        # Resolve the already-tested host binary before hiding source assets.
        # Its installed run below loads only the supplied copied plugin tree.
        code, output = self.command(
            "trigger-installed-test-binary", ["cargo", "test", "--locked", "--jobs", "2",
                "--manifest-path", str(self.source / "src/clap-adapter/Cargo.toml"),
                "--target-dir", str(self.cargo_target), "--features", "trigger",
                "--test", "trigger_dynamic", "--no-run", "--message-format=json"])
        self.assertEqual(code, 0, output)
        artifacts = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
        executables = [item["executable"] for item in artifacts
                       if item.get("reason") == "compiler-artifact"
                       and item.get("target", {}).get("name") == "trigger_dynamic"
                       and item.get("executable")]
        self.assertEqual(len(executables), 1, output)

        installed = self.fixture_root / "installed"
        installed.mkdir()
        shutil.copy2(default_artifact, installed / default_artifact.name)
        shutil.copy2(artifact, installed / artifact.name)
        shutil.copytree(resources, installed / resources.name)
        shutil.rmtree(editor_input)
        shutil.rmtree(self.source / "examples/patches")
        for relative, contents in expected.items():
            copied = installed / resources.name / relative
            self.assertEqual(copied.read_bytes(), contents, relative)
            self.assertNotIn(str(self.source).encode(), copied.read_bytes(), relative)
        code, output = self.command(
            "trigger-installed-runtime", [CMAKE, "-E", "chdir", str(installed), CMAKE, "-E", "env",
                "DANDRUM_TRIGGER_CLAP_PLUGIN_PATH=" + str(installed / artifact.name),
                executables[0], "--test-threads=1"])
        self.assertEqual(code, 0, output)
        self.assertIn("test result: ok.", output)
        self.assertNotRegex(output, r"test result: ok\. 0 passed;")


class ComparisonBuildTest(EngineBuildFixture):
    def test_comparison_configure_retains_juce_targets(self):
        build = self.work / "comparison"
        code, output = self.command("comparison-configure", [CMAKE, "-S", str(ROOT), "-B", str(build),
            "-DCMAKE_BUILD_TYPE=Release", "-DDANDRUM_NATIVE_ONLY=ON", "-DDANDRUM_ENGINE_ONLY=OFF"])
        self.assertEqual(code, 0, output)
        code, output = self.command("comparison-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        for target in ("dandrum-plugin", "dandrum-sampler-plugin", "dandrum-drum-machine-demo", "dandrum-ffi-smoke"):
            self.assertRegex(output, r"(?m)(?:^|\s)" + target + r"(?:\s|$)")
        code, output = self.command("comparison-inventory", [CTEST, "--test-dir", str(build), "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        names = {test["name"] for test in json.loads(output)["tests"]}
        self.assertTrue({"rust-engine-tests", "cxx-rust-engine-ffi-smoke", "cxx-juce-kernel-source",
                         "native-editor-smoke"}.issubset(names), names)


if __name__ == "__main__":
    unittest.main()
