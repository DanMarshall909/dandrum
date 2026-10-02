cmake_minimum_required(VERSION 3.22)

project(Dandrum VERSION 0.1.0 LANGUAGES C CXX)

enable_testing()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
option(DANDRUM_ENABLE_MUTATION_CTEST "Include the long-running cargo-mutants test in CTest" OFF)
option(DANDRUM_NATIVE_ONLY "Build the plugin with the native JUCE editor and no browser dependencies" OFF)
if(DANDRUM_NATIVE_ONLY)
    set(DANDRUM_BROWSER_ENABLED FALSE)
    set(DANDRUM_EDITOR_ADAPTER dandrum-plugin-native-editor)
else()
    set(DANDRUM_BROWSER_ENABLED TRUE)
    set(DANDRUM_EDITOR_ADAPTER dandrum-plugin-web-adapter)
endif()

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
    NAME retired-engine-ffi-symbols
    COMMAND ${CMAKE_COMMAND}
        -DNM=${CMAKE_NM}
        -DRUST_ENGINE_LIB=${RUST_ENGINE_LIB}
        -P ${CMAKE_CURRENT_SOURCE_DIR}/tests/cmake/RetiredEngineFfiSymbols.cmake)

add_test(
    NAME retired-kernel-fields
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/scripts/check-kernel-retirement.py)

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

add_executable(dandrum-sound-lab-controller-test
    tests/cpp/SoundLabControllerTest.cpp
    src/juce-plugin/SoundLabController.cpp)
target_include_directories(dandrum-sound-lab-controller-test PRIVATE src/juce-wrapper src/juce-plugin)
target_link_libraries(dandrum-sound-lab-controller-test PRIVATE dandrum_engine dl pthread m)
add_test(NAME sound-lab-controller COMMAND dandrum-sound-lab-controller-test)

add_executable(dandrum-plugin-meter-capture-test tests/cpp/PluginMeterCaptureTest.cpp)
target_include_directories(dandrum-plugin-meter-capture-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-meter-capture COMMAND dandrum-plugin-meter-capture-test)

add_executable(dandrum-plugin-meter-aggregation-test tests/cpp/PluginMeterAggregationTest.cpp)
target_include_directories(dandrum-plugin-meter-aggregation-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-meter-aggregation COMMAND dandrum-plugin-meter-aggregation-test)

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

add_executable(dandrum-juce-kernel-source-test
    tests/cpp/JuceKernelSourceTest.cpp
    src/juce-wrapper/RustEngineSource.cpp)
target_compile_definitions(dandrum-juce-kernel-source-test PRIVATE
    DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}" JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
target_link_libraries(dandrum-juce-kernel-source-test
    PRIVATE dandrum_engine juce::juce_audio_basics juce::juce_audio_devices juce::juce_core
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
add_test(NAME cxx-juce-kernel-source COMMAND dandrum-juce-kernel-source-test)

add_library(dandrum-plugin-ui-common INTERFACE)
target_sources(dandrum-plugin-ui-common INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentDemoConfiguration.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentFileWatcher.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginFactory.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginProcessor.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/SoundLabController.cpp)

add_library(dandrum-plugin-web-adapter INTERFACE)
target_sources(dandrum-plugin-web-adapter INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentHostWebBridge.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginEditor.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginWebEditorFactory.cpp)
target_link_libraries(dandrum-plugin-web-adapter INTERFACE juce::juce_gui_extra)

add_library(dandrum-plugin-native-editor INTERFACE)
target_sources(dandrum-plugin-native-editor INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginNativeEditor.cpp)
target_link_libraries(dandrum-plugin-native-editor INTERFACE juce::juce_gui_basics)

juce_add_binary_data(dandrum-tb303-instrument-resources
    NAMESPACE Tb303InstrumentBinaryData
    HEADER_NAME Tb303InstrumentBinaryData.h
    SOURCES
        examples/patches/tb303-acid.yaml
        examples/sound-design/tb303-acid-poc.yaml)

juce_add_plugin(dandrum-plugin
    PLUGIN_MANUFACTURER_CODE Dndr
    PLUGIN_CODE Dndm
    FORMATS AU VST3 Standalone
    IS_SYNTH TRUE
    NEEDS_MIDI_INPUT TRUE
    NEEDS_MIDI_OUTPUT FALSE
    NEEDS_WEBVIEW2 ${DANDRUM_BROWSER_ENABLED}
    NEEDS_WEB_BROWSER ${DANDRUM_BROWSER_ENABLED}
    PRODUCT_NAME "Dandrum")

if(NOT DANDRUM_NATIVE_ONLY)
    juce_add_binary_data(dandrum-tb303-react-resources
        NAMESPACE Tb303ReactBinaryData
        HEADER_NAME Tb303ReactBinaryData.h
        SOURCES
            web/tb303/dist/index.html
            web/tb303/dist/app.js
            web/tb303/dist/app.css)
    target_link_libraries(dandrum-plugin PRIVATE dandrum-tb303-react-resources)
    target_compile_definitions(dandrum-plugin PRIVATE DANDRUM_EMBED_TB303_REACT=1)
endif()
target_include_directories(dandrum-plugin PRIVATE src/juce-wrapper)
target_compile_definitions(dandrum-plugin PRIVATE
    DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}"
    DANDRUM_EMBED_TB303_ASSETS=1
    JUCE_USE_CURL=0
    JUCE_WEB_BROWSER=$<BOOL:${DANDRUM_BROWSER_ENABLED}>
    JUCE_USE_WIN_WEBVIEW2=$<BOOL:${DANDRUM_BROWSER_ENABLED}>)
target_link_libraries(dandrum-plugin
    PRIVATE dandrum_engine dandrum-tb303-instrument-resources
        dandrum-plugin-ui-common ${DANDRUM_EDITOR_ADAPTER}
        juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)

juce_add_binary_data(dandrum-sampler-resources
    NAMESPACE SamplerBinaryData
    HEADER_NAME SamplerBinaryData.h
    SOURCES
        examples/patches/advanced-drum-kit.yaml
        examples/patches/assets/advanced-drums.wav)

juce_add_plugin(dandrum-sampler-plugin
    PLUGIN_MANUFACTURER_CODE Dndr
    PLUGIN_CODE Dnsm
    FORMATS AU VST3 Standalone
    IS_SYNTH TRUE
    NEEDS_MIDI_INPUT TRUE
    NEEDS_MIDI_OUTPUT FALSE
    NEEDS_WEBVIEW2 ${DANDRUM_BROWSER_ENABLED}
    NEEDS_WEB_BROWSER ${DANDRUM_BROWSER_ENABLED}
    PRODUCT_NAME "Dandrum Sampler")
target_include_directories(dandrum-sampler-plugin PRIVATE src/juce-wrapper)
target_compile_definitions(dandrum-sampler-plugin PRIVATE
    DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}"
    DANDRUM_DEFAULT_SAMPLER_PLUGIN=1
    DANDRUM_EMBED_SAMPLER_ASSETS=1
    JUCE_USE_CURL=0
    JUCE_WEB_BROWSER=$<BOOL:${DANDRUM_BROWSER_ENABLED}>
    JUCE_USE_WIN_WEBVIEW2=$<BOOL:${DANDRUM_BROWSER_ENABLED}>)
target_link_libraries(dandrum-sampler-plugin
    PRIVATE dandrum_engine dandrum-sampler-resources dandrum-plugin-ui-common ${DANDRUM_EDITOR_ADAPTER}
        juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)

if(DANDRUM_NATIVE_ONLY)
    add_executable(dandrum-native-editor-smoke-test tests/cpp/NativeEditorSmokeTest.cpp)
    target_include_directories(dandrum-native-editor-smoke-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-native-editor-smoke-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
    target_link_libraries(dandrum-native-editor-smoke-test
        PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
            juce::juce_audio_utils juce::juce_gui_basics
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME native-editor-smoke COMMAND dandrum-native-editor-smoke-test)
endif()

if(NOT DANDRUM_NATIVE_ONLY)
    add_executable(dandrum-sampler-plugin-host-test tests/cpp/PluginSamplerHostTest.cpp)
    target_include_directories(dandrum-sampler-plugin-host-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-sampler-plugin-host-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-sampler-plugin-host-test
        PRIVATE dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-sampler-plugin-host COMMAND dandrum-sampler-plugin-host-test)

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

    add_executable(dandrum-plugin-editor-bridge-test tests/cpp/PluginEditorBridgeTest.cpp)
    target_include_directories(dandrum-plugin-editor-bridge-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-editor-bridge-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-editor-bridge-test
        PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-editor-bridge COMMAND dandrum-plugin-editor-bridge-test)

    add_executable(dandrum-plugin-demo-configuration-test tests/cpp/PluginDemoConfigurationTest.cpp)
    target_include_directories(dandrum-plugin-demo-configuration-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-demo-configuration-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-demo-configuration-test
        PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-demo-configuration COMMAND dandrum-plugin-demo-configuration-test)
endif()

add_test(NAME spec-coverage-map
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_spec_coverage_map.py)

add_test(NAME rust-coverage-gate
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_rust_coverage_gate.py)

if(NOT DANDRUM_NATIVE_ONLY)
    find_program(DANDRUM_NODE_EXECUTABLE node)
    if(NOT DANDRUM_NODE_EXECUTABLE OR NOT EXISTS "${DANDRUM_NODE_EXECUTABLE}")
        message(FATAL_ERROR "Node.js is required to run the WebView page and native bridge tests")
    endif()
    add_test(NAME shared-instrument-ui COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/SharedInstrumentUiTest.mjs)
    add_test(NAME tb303-page COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/Tb303PageTest.mjs)
    add_test(NAME tb303-react-model COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/tb303/src/model.test.mjs)
    add_test(NAME tb303-react-audition COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/tb303/src/note-audition.test.mjs)
    add_test(NAME native-function-bootstrap COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/NativeFunctionBootstrapTest.mjs)

    add_executable(dandrum-plugin-file-watch-reload-test tests/cpp/PluginFileWatchReloadTest.cpp)
    target_include_directories(dandrum-plugin-file-watch-reload-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-file-watch-reload-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-file-watch-reload-test
        PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-file-watch-reload COMMAND dandrum-plugin-file-watch-reload-test)
endif()

# These plain test executables compile JUCE's web-browser module themselves.
# Unlike juce_add_plugin(), add_executable() does not automatically attach the
# Linux WebKit/GTK usage requirements requested by NEEDS_WEB_BROWSER.
if(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND NOT DANDRUM_NATIVE_ONLY)
    foreach(plugin_test_target
            dandrum-plugin-construction-test
            dandrum-plugin-host-parameter-surface-test
            dandrum-plugin-editor-bridge-test
            dandrum-plugin-demo-configuration-test
            dandrum-plugin-file-watch-reload-test)
        target_include_directories(${plugin_test_target} PRIVATE
            ${CMAKE_CURRENT_BINARY_DIR}/dandrum-plugin_artefacts/JuceLibraryCode)
        target_link_libraries(${plugin_test_target} PRIVATE juce::pkgconfig_JUCE_BROWSER_LINUX_DEPS)
    endforeach()
    target_include_directories(dandrum-sampler-plugin-host-test PRIVATE
        ${CMAKE_CURRENT_BINARY_DIR}/dandrum-sampler-plugin_artefacts/JuceLibraryCode
        ${CMAKE_CURRENT_BINARY_DIR}/dandrum-plugin_artefacts/JuceLibraryCode)
    target_link_libraries(dandrum-sampler-plugin-host-test PRIVATE juce::pkgconfig_JUCE_BROWSER_LINUX_DEPS)
endif()

add_executable(dandrum-drum-loop-pattern-test tests/cpp/DrumLoopPatternTest.cpp)
target_include_directories(dandrum-drum-loop-pattern-test PRIVATE src/juce-wrapper)
target_link_libraries(dandrum-drum-loop-pattern-test PRIVATE)
add_test(NAME drum-loop-pattern-test COMMAND dandrum-drum-loop-pattern-test)
