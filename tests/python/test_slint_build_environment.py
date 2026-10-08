"""Exercise the Slint compiler environment through real GNU Make and Cargo."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CARGO = shutil.which('cargo') or str(Path.home() / '.cargo/bin/cargo')


@unittest.skipUnless(sys.platform.startswith("linux"), "GNU Make regression is Linux-specific")
class SlintBuildEnvironmentTest(unittest.TestCase):
    def test_nested_make_survives_outer_silent_flags(self):
        with tempfile.TemporaryDirectory(prefix='dandrum Slint Make ') as directory:
            root = Path(directory)
            config = root / 'sdk/lib/cmake/Slint'
            config.mkdir(parents=True)
            (config / 'SlintConfig.cmake').write_text('''
function(corrosion_set_env_vars target_name)
  set_property(TARGET ${target_name} APPEND PROPERTY CORROSION_ENVIRONMENT_VARIABLES ${ARGN})
endfunction()
add_custom_target(slint-compiler
  COMMAND ${CMAKE_COMMAND} -E env
    "$<TARGET_PROPERTY:slint-compiler,CORROSION_ENVIRONMENT_VARIABLES>"
    "CARGO_BUILD_JOBS=1" "CARGO_TARGET_DIR=${CMAKE_BINARY_DIR}/cargo"
    "${PROBE_CARGO}" build --manifest-path "${CMAKE_SOURCE_DIR}/probe/Cargo.toml"
  COMMAND_EXPAND_LISTS)
''')
            crate = root / 'probe'
            (crate / 'src').mkdir(parents=True)
            (crate / 'Cargo.toml').write_text('[package]\nname="slint_make_probe"\nversion="0.1.0"\nedition="2021"\n')
            (crate / 'src/main.rs').write_text('fn main() {}\n')
            # This is the same nested Make contract used by jemalloc's build.rs.
            (crate / 'build.rs').write_text('''
fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let file = out.join("nested.mk");
    std::fs::write(&file, "all:\\n\\t@echo nested Make succeeded\\n").unwrap();
    let result = std::process::Command::new("make").arg("-f").arg(&file)
        .env("MAKEFLAGS", format!("{} {}", std::env::var("CARGO_MAKEFLAGS").unwrap(),
            std::env::var("MAKEFLAGS").unwrap_or_default()))
        .status().unwrap();
    assert!(result.success(), "nested Make inherited malformed outer flags");
    std::fs::write(out.join("nested-make-ok.txt"), "passed").unwrap();
}
''')
            self.configure(root)
            env = dict(os.environ, MAKEFLAGS='s', MFLAGS='-s')
            result = subprocess.run(['cmake', '--build', str(root / 'build'), '--target', 'slint-compiler'],
                                    env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(len(list((root / 'build/cargo/debug/build').glob('*/out/nested-make-ok.txt'))), 1)

    def test_prebuilt_slint_does_not_require_corrosion(self):
        with tempfile.TemporaryDirectory(prefix='dandrum Slint SDK ') as directory:
            root = Path(directory)
            config = root / 'sdk/lib/cmake/Slint'
            config.mkdir(parents=True)
            (config / 'SlintConfig.cmake').write_text('add_library(Slint::Slint INTERFACE IMPORTED)\n')
            self.configure(root)

    def configure(self, root):
        (root / 'CMakeLists.txt').write_text(f'''cmake_minimum_required(VERSION 3.22)
project(SlintEnvironmentProbe LANGUAGES NONE)
set(FETCHCONTENT_TRY_FIND_PACKAGE_MODE ALWAYS)
list(APPEND CMAKE_PREFIX_PATH "{root / 'sdk'}")
set(PROBE_CARGO "{CARGO}")
include("{ROOT / 'cmake/SlintDependency.cmake'}")
''')
        result = subprocess.run(['cmake', '-S', str(root), '-B', str(root / 'build'), '-G', 'Unix Makefiles'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
