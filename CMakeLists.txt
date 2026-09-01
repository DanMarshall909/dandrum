cmake_minimum_required(VERSION 3.22)

project(Dandrum VERSION 0.1.0 LANGUAGES C CXX)

enable_testing()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
option(DANDRUM_ENABLE_MUTATION_CTEST "Include the long-running cargo-mutants test in CTest" OFF)

set(RUST_ENGINE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/rust-engine)
set(RUST_ENGINE_LIB ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_engine.a)
set(CARGO_BIN $ENV{HOME}/.cargo/bin/cargo)
set(RUST_ENGINE_MUTATION_EXCLUDE_ARGS
    --exclude src/bin/*.rs
    --exclude src/cli.rs)

file(GLOB_RECURSE RUST_ENGINE_SOURCES CONFIGURE_DEPENDS ${RUST_ENGINE_DIR}/src/*.rs)

add_custom_command(
    OUTPUT ${RUST_ENGINE_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target ${CARGO_BIN} build --release
    WORKING_DIRECTORY ${RUST_ENGINE_DIR}
    DEPENDS
        ${RUST_ENGINE_DIR}/Cargo.toml
        ${RUST_ENGINE_SOURCES}
    COMMENT "Building Rust audio engine")

add_custom_target(dandrum-engine-rust DEPENDS ${RUST_ENGINE_LIB})

add_test(
    NAME rust-engine-tests
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target ${CARGO_BIN} test
    WORKING_DIRECTORY ${RUST_ENGINE_DIR})

if(DANDRUM_ENABLE_MUTATION_CTEST)
    add_test(
        NAME rust-engine-mutation-tests
        COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target ${CARGO_BIN} mutants
            ${RUST_ENGINE_MUTATION_EXCLUDE_ARGS}
        WORKING_DIRECTORY ${RUST_ENGINE_DIR})
    set_tests_properties(rust-engine-mutation-tests PROPERTIES TIMEOUT 1200)
endif()

add_library(dandrum_engine STATIC IMPORTED GLOBAL)
set_target_properties(dandrum_engine PROPERTIES IMPORTED_LOCATION ${RUST_ENGINE_LIB})
add_dependencies(dandrum_engine dandrum-engine-rust)

add_executable(dandrum-ffi-smoke tests/cpp/RustEngineFfiSmoke.cpp)
target_include_directories(dandrum-ffi-smoke PRIVATE src/juce-wrapper)
target_link_libraries(dandrum-ffi-smoke PRIVATE dandrum_engine dl pthread m)
add_test(NAME cxx-rust-engine-ffi-smoke COMMAND dandrum-ffi-smoke)

add_executable(dandrum-sample-accurate-midi-smoke tests/cpp/RustEngineSampleAccurateMidiSmoke.cpp)
target_include_directories(dandrum-sample-accurate-midi-smoke PRIVATE src/juce-wrapper)
target_link_libraries(dandrum-sample-accurate-midi-smoke PRIVATE dandrum_engine dl pthread m)
add_test(NAME cxx-rust-engine-sample-accurate-midi-smoke COMMAND dandrum-sample-accurate-midi-smoke)

add_test(
    NAME realtime-callback-safety
    COMMAND ${CMAKE_COMMAND}
        -DSOURCE_ROOT=${CMAKE_CURRENT_SOURCE_DIR}
        -P ${CMAKE_CURRENT_SOURCE_DIR}/tests/cmake/RealtimeCallbackSafety.cmake)

add_executable(dandrum-tb303-web-ui-contract-test tests/cpp/Tb303WebUiContractTest.cpp)
target_include_directories(dandrum-tb303-web-ui-contract-test PRIVATE src/juce-plugin)
add_test(NAME tb303-web-ui-contract COMMAND dandrum-tb303-web-ui-contract-test)

add_subdirectory(third_party/JUCE)

juce_add_console_app(dandrum-drum-machine-demo PRODUCT_NAME "Dandrum Drum Machine Demo")
target_sources(dandrum-drum-machine-demo PRIVATE
    src/juce-wrapper/Cli.cpp
    src/juce-wrapper/DrumLoop.h
    src/juce-wrapper/Main.cpp
    src/juce-wrapper/MidiToRustEngine.cpp
    src/juce-wrapper/RustEngineSource.cpp)
target_compile_definitions(dandrum-drum-machine-demo PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
target_link_libraries(dandrum-drum-machine-demo
    PRIVATE dandrum_engine juce::juce_audio_basics juce::juce_audio_devices juce::juce_core
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)

juce_add_plugin(dandrum-plugin
    PLUGIN_MANUFACTURER_CODE Dndr
    PLUGIN_CODE Dndm
    FORMATS AU VST3 Standalone
    IS_SYNTH TRUE
    NEEDS_MIDI_INPUT TRUE
    NEEDS_MIDI_OUTPUT FALSE
    NEEDS_WEBVIEW2 TRUE
    NEEDS_WEB_BROWSER TRUE
    PRODUCT_NAME "Dandrum")

target_sources(dandrum-plugin PRIVATE
    src/juce-plugin/InstrumentFileWatcher.cpp
    src/juce-plugin/PluginEditor.cpp
    src/juce-plugin/PluginProcessor.cpp)
target_include_directories(dandrum-plugin PRIVATE src/juce-wrapper)
target_compile_definitions(dandrum-plugin PRIVATE
    JUCE_USE_CURL=0
    JUCE_WEB_BROWSER=1
    JUCE_USE_WIN_WEBVIEW2=1)
target_link_libraries(dandrum-plugin
    PRIVATE dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)

add_executable(dandrum-plugin-construction-test tests/cpp/PluginConstructionTest.cpp)
target_include_directories(dandrum-plugin-construction-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-construction-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
target_link_libraries(dandrum-plugin-construction-test
    PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
add_test(NAME cxx-plugin-construction COMMAND dandrum-plugin-construction-test)

add_executable(dandrum-plugin-host-parameter-surface-test tests/cpp/PluginHostParameterSurfaceTest.cpp)
target_include_directories(dandrum-plugin-host-parameter-surface-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-host-parameter-surface-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
target_link_libraries(dandrum-plugin-host-parameter-surface-test
    PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
add_test(NAME cxx-plugin-host-parameter-surface COMMAND dandrum-plugin-host-parameter-surface-test)

add_executable(dandrum-plugin-file-watch-reload-test tests/cpp/PluginFileWatchReloadTest.cpp)
target_include_directories(dandrum-plugin-file-watch-reload-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-file-watch-reload-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
target_link_libraries(dandrum-plugin-file-watch-reload-test
    PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
add_test(NAME cxx-plugin-file-watch-reload COMMAND dandrum-plugin-file-watch-reload-test)

add_executable(dandrum-drum-loop-pattern-test tests/cpp/DrumLoopPatternTest.cpp)
target_include_directories(dandrum-drum-loop-pattern-test PRIVATE src/juce-wrapper)
target_link_libraries(dandrum-drum-loop-pattern-test PRIVATE)
add_test(NAME drum-loop-pattern-test COMMAND dandrum-drum-loop-pattern-test)
