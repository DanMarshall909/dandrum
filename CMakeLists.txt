cmake_minimum_required(VERSION 3.22)

project(Dandrum VERSION 0.1.0 LANGUAGES C CXX)

enable_testing()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
option(DANDRUM_ENABLE_MUTATION_CTEST "Include the long-running cargo-mutants test in CTest" OFF)
option(DANDRUM_NATIVE_ONLY "Build the plugin with the native JUCE editor and no browser dependencies" OFF)
option(DANDRUM_BUILD_FILTER_UI_SPIKES "Build optional Slint and JIVE filter experiments" OFF)
option(DANDRUM_BUILD_TRIGGER_SLINT "Build the silent Trigger Slint performance preview" OFF)
if(DANDRUM_NATIVE_ONLY)
    set(DANDRUM_BROWSER_ENABLED FALSE)
    set(DANDRUM_EDITOR_ADAPTER dandrum-plugin-native-editor)
else()
    set(DANDRUM_BROWSER_ENABLED TRUE)
    set(DANDRUM_EDITOR_ADAPTER dandrum-plugin-web-adapter)
    find_program(DANDRUM_NODE_EXECUTABLE node)
    if(NOT DANDRUM_NODE_EXECUTABLE OR NOT EXISTS "${DANDRUM_NODE_EXECUTABLE}")
        message(FATAL_ERROR "Node.js is required to run the WebView page and native bridge tests")
    endif()
    execute_process(
        COMMAND "${DANDRUM_NODE_EXECUTABLE}" -e
            "const appRequire = require('node:module').createRequire(process.argv[1]); appRequire('react'); appRequire('react-dom/server');"
            "${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/package.json"
        RESULT_VARIABLE DANDRUM_REACT_TEST_SETUP_RESULT
        OUTPUT_QUIET ERROR_QUIET)
    if(NOT DANDRUM_REACT_TEST_SETUP_RESULT EQUAL 0)
        message(FATAL_ERROR
            "React rendering test dependencies are unavailable. Run npm ci --prefix web/sampler from the repository root, then configure again.")
    endif()
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

add_executable(dandrum-plugin-ui-command-test tests/cpp/PluginUiCommandTest.cpp)
target_include_directories(dandrum-plugin-ui-command-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-ui-command COMMAND dandrum-plugin-ui-command-test)

add_executable(dandrum-plugin-meter-capture-test tests/cpp/PluginMeterCaptureTest.cpp)
target_include_directories(dandrum-plugin-meter-capture-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-meter-capture COMMAND dandrum-plugin-meter-capture-test)

add_executable(dandrum-plugin-live-capture-test tests/cpp/PluginLiveCaptureTest.cpp)
target_include_directories(dandrum-plugin-live-capture-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-live-capture COMMAND dandrum-plugin-live-capture-test)

add_executable(dandrum-plugin-meter-aggregation-test tests/cpp/PluginMeterAggregationTest.cpp)
target_include_directories(dandrum-plugin-meter-aggregation-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-meter-aggregation COMMAND dandrum-plugin-meter-aggregation-test)

add_executable(dandrum-plugin-meter-delivery-test tests/cpp/PluginMeterDeliveryTest.cpp)
target_include_directories(dandrum-plugin-meter-delivery-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-meter-delivery COMMAND dandrum-plugin-meter-delivery-test)

add_executable(dandrum-plugin-meter-display-test tests/cpp/PluginMeterDisplayTest.cpp)
target_include_directories(dandrum-plugin-meter-display-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-meter-display COMMAND dandrum-plugin-meter-display-test)

add_executable(dandrum-plugin-waveform-service-test
    tests/cpp/PluginWaveformServiceTest.cpp
    src/juce-plugin/InstrumentUiWaveformService.cpp)
target_include_directories(dandrum-plugin-waveform-service-test PRIVATE
    src/juce-wrapper src/juce-plugin)
target_link_libraries(dandrum-plugin-waveform-service-test PRIVATE dandrum_engine dl pthread m)
add_test(NAME cxx-plugin-waveform-service COMMAND dandrum-plugin-waveform-service-test)

add_executable(dandrum-plugin-waveform-geometry-test
    tests/cpp/PluginWaveformGeometryTest.cpp)
target_include_directories(dandrum-plugin-waveform-geometry-test PRIVATE src/juce-plugin)
add_test(NAME cxx-plugin-waveform-geometry COMMAND dandrum-plugin-waveform-geometry-test)

add_executable(dandrum-plugin-spectral-geometry-test tests/cpp/PluginSpectralGeometryTest.cpp)
target_include_directories(dandrum-plugin-spectral-geometry-test PRIVATE src/juce-wrapper src/juce-plugin)
add_test(NAME cxx-plugin-spectral-geometry COMMAND dandrum-plugin-spectral-geometry-test)

add_subdirectory(third_party/JUCE)

add_executable(dandrum-filter-spike-engine-test tests/cpp/FilterSpikeEngineTest.cpp)
target_include_directories(dandrum-filter-spike-engine-test PRIVATE src/juce-wrapper)
target_compile_definitions(dandrum-filter-spike-engine-test PRIVATE DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}")
target_link_libraries(dandrum-filter-spike-engine-test PRIVATE dandrum_engine dl pthread m)
add_test(NAME filter-spike-engine COMMAND dandrum-filter-spike-engine-test)

if(DANDRUM_BUILD_FILTER_UI_SPIKES)
    include(cmake/FilterUiDependencies.cmake)
    juce_add_plugin(dandrum-filter-slint
        COMPANY_NAME Dandrum PLUGIN_MANUFACTURER_CODE Dndr PLUGIN_CODE FlSl
        FORMATS VST3 Standalone IS_SYNTH FALSE NEEDS_MIDI_INPUT FALSE NEEDS_MIDI_OUTPUT FALSE
        PRODUCT_NAME "Dandrum Filter Slint")
    juce_add_plugin(dandrum-filter-jive
        COMPANY_NAME Dandrum PLUGIN_MANUFACTURER_CODE Dndr PLUGIN_CODE FlJv
        FORMATS VST3 Standalone IS_SYNTH FALSE NEEDS_MIDI_INPUT FALSE NEEDS_MIDI_OUTPUT FALSE
        PRODUCT_NAME "Dandrum Filter JIVE")
    include(cmake/FilterUiTargets.cmake)
endif()

if(DANDRUM_BUILD_TRIGGER_SLINT)
    include(cmake/SlintDependency.cmake)
    juce_add_gui_app(dandrum-trigger-slint PRODUCT_NAME "Dandrum Trigger Slint")
    include(cmake/TriggerSlintTarget.cmake)
endif()

add_executable(dandrum-plugin-spectral-service-test
    tests/cpp/PluginSpectralServiceTest.cpp
    src/juce-plugin/InstrumentUiSpectralService.cpp
    src/juce-plugin/InstrumentUiSpectrumAnalysis.cpp)
target_include_directories(dandrum-plugin-spectral-service-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-spectral-service-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
target_link_libraries(dandrum-plugin-spectral-service-test PRIVATE dandrum_engine juce::juce_dsp dl pthread m)
add_test(NAME cxx-plugin-spectral-service COMMAND dandrum-plugin-spectral-service-test)

add_executable(dandrum-plugin-live-analysis-test
    tests/cpp/PluginLiveAnalysisTest.cpp
    src/juce-plugin/InstrumentUiLiveAnalysis.cpp
    src/juce-plugin/InstrumentUiSpectrumAnalysis.cpp)
target_include_directories(dandrum-plugin-live-analysis-test PRIVATE src/juce-plugin)
target_compile_definitions(dandrum-plugin-live-analysis-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
target_link_libraries(dandrum-plugin-live-analysis-test PRIVATE juce::juce_dsp dl pthread m)
add_test(NAME cxx-plugin-live-analysis COMMAND dandrum-plugin-live-analysis-test)

add_executable(dandrum-plugin-live-service-test
    tests/cpp/PluginLiveServiceTest.cpp
    src/juce-plugin/InstrumentUiLiveService.cpp
    src/juce-plugin/InstrumentUiLiveAnalysis.cpp
    src/juce-plugin/InstrumentUiSpectrumAnalysis.cpp)
target_include_directories(dandrum-plugin-live-service-test PRIVATE src/juce-plugin)
target_compile_definitions(dandrum-plugin-live-service-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
target_link_libraries(dandrum-plugin-live-service-test PRIVATE juce::juce_dsp dl pthread m)
add_test(NAME cxx-plugin-live-service COMMAND dandrum-plugin-live-service-test)

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
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentUiWaveformService.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentUiSpectralService.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentUiSpectrumAnalysis.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentUiLiveAnalysis.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentUiLiveService.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginFactory.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginProcessor.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/SoundLabController.cpp)
target_link_libraries(dandrum-plugin-ui-common INTERFACE juce::juce_dsp)

add_library(dandrum-plugin-web-adapter INTERFACE)
target_sources(dandrum-plugin-web-adapter INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/InstrumentHostWebBridge.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginEditor.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginWebEditorFactory.cpp)
target_link_libraries(dandrum-plugin-web-adapter INTERFACE juce::juce_gui_extra)

add_library(dandrum-plugin-native-editor INTERFACE)
target_sources(dandrum-plugin-native-editor INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/PluginNativeEditor.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/juce-plugin/NativeOutputBuses.cpp)
juce_add_binary_data(dandrum-native-knob-fonts
    NAMESPACE NativeKnobFontBinaryData
    HEADER_NAME NativeKnobFontBinaryData.h
    SOURCES
        ui/design-system/fonts/BarlowSemiCondensed/BarlowSemiCondensed-SemiBold.ttf
        ui/design-system/fonts/JetBrainsMono/JetBrainsMono-Medium.ttf
        ui/design-system/fonts/BarlowSemiCondensed/OFL.txt
        ui/design-system/fonts/JetBrainsMono/OFL.txt)
target_link_libraries(dandrum-plugin-native-editor INTERFACE
    juce::juce_gui_basics dandrum-native-knob-fonts)

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
    juce_add_binary_data(dandrum-sampler-react-resources
        NAMESPACE SamplerReactBinaryData
        HEADER_NAME SamplerReactBinaryData.h
        SOURCES
            web/sampler/dist/index.html
            web/sampler/dist/app.js
            web/sampler/dist/app.css)
    target_link_libraries(dandrum-plugin PRIVATE dandrum-sampler-react-resources)
    target_compile_definitions(dandrum-plugin PRIVATE DANDRUM_EMBED_SAMPLER_REACT=1)
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
if(NOT DANDRUM_NATIVE_ONLY)
    target_link_libraries(dandrum-sampler-plugin PRIVATE dandrum-sampler-react-resources)
    target_compile_definitions(dandrum-sampler-plugin PRIVATE DANDRUM_EMBED_SAMPLER_REACT=1)
endif()
if(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND DANDRUM_BROWSER_ENABLED)
    # Both JUCE plugin targets generate the shared browser helper in the first
    # target's directory. Serialize them so parallel Make cannot rename it twice.
    add_dependencies(dandrum-sampler-plugin dandrum-plugin)
endif()

if(DANDRUM_NATIVE_ONLY)
    add_executable(dandrum-native-editor-smoke-test tests/cpp/NativeEditorSmokeTest.cpp)
    target_include_directories(dandrum-native-editor-smoke-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-native-editor-smoke-test PRIVATE
        JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0 JUCE_MODAL_LOOPS_PERMITTED=1)
    target_link_libraries(dandrum-native-editor-smoke-test
        PRIVATE dandrum-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
            juce::juce_audio_utils juce::juce_gui_basics
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME native-editor-smoke COMMAND dandrum-native-editor-smoke-test)
    add_test(NAME native-meter-characterization COMMAND dandrum-native-editor-smoke-test --meter-characterization)
    add_test(NAME native-output-buses COMMAND dandrum-native-editor-smoke-test --output-buses)
    if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
        target_compile_definitions(dandrum-native-editor-smoke-test PRIVATE
            DANDRUM_TEST_OUTPUT_RELOAD_INTERLEAVING=1)
        target_link_options(dandrum-native-editor-smoke-test PRIVATE
            "LINKER:--wrap=_ZNK21DandrumAudioProcessor21getPreparedUiDocumentEv")
        add_test(NAME native-output-reload-interleaving
            COMMAND dandrum-native-editor-smoke-test --output-reload-interleaving)
        set_tests_properties(native-output-reload-interleaving PROPERTIES
            LABELS "runtime" TIMEOUT 15 RUN_SERIAL TRUE)
        # Link wrappers hold the real Rust reduction at a deterministic barrier
        # without adding test hooks to the processor or either editor.
        add_executable(dandrum-native-waveform-lifetime-test tests/cpp/NativeWaveformLifetimeTest.cpp)
        target_include_directories(dandrum-native-waveform-lifetime-test PRIVATE src/juce-wrapper src/juce-plugin)
        target_compile_definitions(dandrum-native-waveform-lifetime-test PRIVATE
            JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0 JUCE_MODAL_LOOPS_PERMITTED=1)
        target_link_libraries(dandrum-native-waveform-lifetime-test PRIVATE
            dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
            juce::juce_audio_utils juce::juce_gui_basics
            juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
        target_link_options(dandrum-native-waveform-lifetime-test PRIVATE
            "LINKER:--wrap=dandrum_kernel_waveform_reduce"
            "LINKER:--wrap=dandrum_kernel_waveform_source_destroy"
            "LINKER:--wrap=dandrum_kernel_destroy")
        add_test(NAME native-waveform-lifetime COMMAND dandrum-native-waveform-lifetime-test)
        set_tests_properties(native-waveform-lifetime PROPERTIES
            LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 30 RUN_SERIAL TRUE)
    endif()
endif()

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    add_executable(dandrum-plugin-spectral-job-test tests/cpp/PluginSpectralJobTest.cpp)
    target_include_directories(dandrum-plugin-spectral-job-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-spectral-job-test PRIVATE
        JUCE_USE_CURL=0 JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
        JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
    target_link_libraries(dandrum-plugin-spectral-job-test PRIVATE
        dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
        juce::juce_audio_utils juce::juce_dsp
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    target_link_options(dandrum-plugin-spectral-job-test PRIVATE
        "LINKER:--wrap=dandrum_kernel_prepared_source_copy_channel"
        "LINKER:--wrap=dandrum_kernel_waveform_source_destroy")
    add_test(NAME cxx-plugin-spectral-job COMMAND dandrum-plugin-spectral-job-test)
    set_tests_properties(cxx-plugin-spectral-job PROPERTIES LABELS "runtime" TIMEOUT 30 RUN_SERIAL TRUE)
endif()

add_executable(dandrum-plugin-knob-parity-test tests/cpp/PluginKnobParityTest.cpp)
target_include_directories(dandrum-plugin-knob-parity-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-knob-parity-test PRIVATE
    JUCE_USE_CURL=0 JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
    JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
target_link_libraries(dandrum-plugin-knob-parity-test PRIVATE
    dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
    juce::juce_audio_utils juce::juce_gui_basics
    juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
if(DANDRUM_NATIVE_ONLY)
    add_test(NAME native-knob-parity COMMAND dandrum-plugin-knob-parity-test)
    set_tests_properties(native-knob-parity PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
else()
    target_link_libraries(dandrum-plugin-knob-parity-test PRIVATE juce::juce_gui_extra)
    add_test(NAME web-knob-parity COMMAND dandrum-plugin-knob-parity-test)
    set_tests_properties(web-knob-parity PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
endif()

add_executable(dandrum-plugin-live-processor-test tests/cpp/PluginLiveProcessorTest.cpp)
target_include_directories(dandrum-plugin-live-processor-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-live-processor-test PRIVATE
    DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}"
    JUCE_USE_CURL=0 JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
    JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
target_link_libraries(dandrum-plugin-live-processor-test PRIVATE
    dandrum-sampler-plugin dandrum_engine juce::juce_audio_utils juce::juce_dsp
    juce::juce_recommended_config_flags juce::juce_recommended_warning_flags)
if(NOT DANDRUM_NATIVE_ONLY)
    target_link_libraries(dandrum-plugin-live-processor-test PRIVATE juce::juce_gui_extra)
endif()
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    target_link_options(dandrum-plugin-live-processor-test PRIVATE
        "LINKER:--wrap=pthread_mutex_lock" "LINKER:--wrap=_ZNSt6thread4joinEv"
        "LINKER:--wrap=_ZNK4juce3dsp3FFT36performFrequencyOnlyForwardTransformEPfb"
        "LINKER:--wrap=dandrum_kernel_destroy" "LINKER:--wrap=dandrum_kernel_prepare_file")
endif()
add_test(NAME cxx-plugin-live-processor COMMAND dandrum-plugin-live-processor-test)
set_tests_properties(cxx-plugin-live-processor PROPERTIES TIMEOUT 15)

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    add_executable(dandrum-plugin-engine-handoff-test tests/cpp/PluginEngineHandoffTest.cpp)
    target_include_directories(dandrum-plugin-engine-handoff-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-engine-handoff-test PRIVATE
        DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}" JUCE_USE_CURL=0
        JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
        JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
    target_link_libraries(dandrum-plugin-engine-handoff-test PRIVATE
        dandrum-sampler-plugin dandrum_engine juce::juce_audio_utils
        juce::juce_recommended_config_flags juce::juce_recommended_warning_flags)
    if(NOT DANDRUM_NATIVE_ONLY)
        target_link_libraries(dandrum-plugin-engine-handoff-test PRIVATE juce::juce_gui_extra)
    endif()
    target_link_options(dandrum-plugin-engine-handoff-test PRIVATE
        "LINKER:--wrap=dandrum_kernel_render" "LINKER:--wrap=dandrum_kernel_destroy"
        "LINKER:--wrap=dandrum_kernel_prepare_file" "LINKER:--wrap=pthread_mutex_lock")
    target_link_options(dandrum-plugin-engine-handoff-test PRIVATE
        "LINKER:--wrap=_ZNSt6thread15_M_start_threadESt10unique_ptrINS_6_StateESt14default_deleteIS1_EEPFvvE"
        "LINKER:--wrap=_ZNSt6thread4joinEv")
    add_test(NAME cxx-plugin-engine-handoff-baseline COMMAND dandrum-plugin-engine-handoff-test)
    add_test(NAME cxx-plugin-engine-handoff COMMAND dandrum-plugin-engine-handoff-test --hold)
    add_test(NAME cxx-plugin-live-handoff COMMAND dandrum-plugin-engine-handoff-test --live)
    add_test(NAME cxx-plugin-automatic-reload COMMAND dandrum-plugin-engine-handoff-test --automatic-job)
    add_test(NAME cxx-plugin-reload-job-contracts COMMAND dandrum-plugin-engine-handoff-test --job-baseline)
    set_tests_properties(cxx-plugin-engine-handoff-baseline cxx-plugin-engine-handoff cxx-plugin-live-handoff PROPERTIES TIMEOUT 15)
    set_tests_properties(cxx-plugin-automatic-reload PROPERTIES TIMEOUT 15)
    set_tests_properties(cxx-plugin-reload-job-contracts PROPERTIES TIMEOUT 15)
endif()

add_executable(dandrum-plugin-waveform-parity-test tests/cpp/PluginWaveformParityTest.cpp)
target_include_directories(dandrum-plugin-waveform-parity-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-waveform-parity-test PRIVATE
    JUCE_USE_CURL=0 JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
    JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
target_link_libraries(dandrum-plugin-waveform-parity-test PRIVATE
    dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
    juce::juce_audio_utils juce::juce_gui_basics
    juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
if(DANDRUM_NATIVE_ONLY)
    add_test(NAME native-waveform-parity COMMAND dandrum-plugin-waveform-parity-test)
    add_test(NAME native-live-renderer COMMAND dandrum-plugin-waveform-parity-test --live)
    add_test(NAME native-output-buses-runtime COMMAND dandrum-plugin-waveform-parity-test --output-buses)
    set_tests_properties(native-output-buses-runtime PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 15 RUN_SERIAL TRUE)
    set_tests_properties(native-live-renderer PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
    set_tests_properties(native-waveform-parity PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
else()
    target_link_libraries(dandrum-plugin-waveform-parity-test PRIVATE juce::juce_gui_extra)
    add_test(NAME web-waveform-parity COMMAND dandrum-plugin-waveform-parity-test)
    add_test(NAME web-live-renderer COMMAND dandrum-plugin-waveform-parity-test --live)
    add_test(NAME web-live-stalled-replies COMMAND dandrum-plugin-waveform-parity-test --live-stall)
    set_tests_properties(web-live-stalled-replies PROPERTIES LABELS "runtime;stress" SKIP_RETURN_CODE 77 TIMEOUT 90 RUN_SERIAL TRUE ENVIRONMENT "NO_AT_BRIDGE=1")
    set_tests_properties(web-live-renderer PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
    set_tests_properties(web-waveform-parity PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
endif()

add_executable(dandrum-plugin-spectral-parity-test tests/cpp/PluginSpectralParityTest.cpp)
target_include_directories(dandrum-plugin-spectral-parity-test PRIVATE src/juce-wrapper src/juce-plugin)
target_compile_definitions(dandrum-plugin-spectral-parity-test PRIVATE
    JUCE_USE_CURL=0 JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
    JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
target_link_libraries(dandrum-plugin-spectral-parity-test PRIVATE
    dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors
    juce::juce_audio_utils juce::juce_gui_basics
    juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    target_link_options(dandrum-plugin-spectral-parity-test PRIVATE
        "LINKER:--wrap=dandrum_kernel_prepared_source_copy_channel")
endif()
if(DANDRUM_NATIVE_ONLY)
    add_test(NAME native-spectral-parity COMMAND dandrum-plugin-spectral-parity-test)
    set_tests_properties(native-spectral-parity PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
else()
    target_link_libraries(dandrum-plugin-spectral-parity-test PRIVATE juce::juce_gui_extra)
    add_test(NAME web-spectral-parity COMMAND dandrum-plugin-spectral-parity-test)
    set_tests_properties(web-spectral-parity PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 60 RUN_SERIAL TRUE)
endif()

if(NOT DANDRUM_NATIVE_ONLY)
    add_executable(dandrum-sampler-plugin-host-test tests/cpp/PluginSamplerHostTest.cpp)
    target_include_directories(dandrum-sampler-plugin-host-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-sampler-plugin-host-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-sampler-plugin-host-test
        PRIVATE dandrum-sampler-plugin dandrum_engine juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-sampler-plugin-host COMMAND dandrum-sampler-plugin-host-test)
    add_test(NAME cxx-output-bus-metadata COMMAND dandrum-sampler-plugin-host-test --output-bus-metadata)

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
    add_test(NAME cxx-plugin-parameter-publication COMMAND dandrum-plugin-editor-bridge-test --parameter-publication)
    add_test(NAME cxx-plugin-live-bridge COMMAND dandrum-plugin-editor-bridge-test --live-bridge)
    set_tests_properties(cxx-plugin-live-bridge PROPERTIES TIMEOUT 15)
    if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
        target_link_options(dandrum-plugin-editor-bridge-test PRIVATE
            "LINKER:--wrap=dandrum_kernel_prepared_source_copy_channel")
    endif()

    add_test(NAME tb303-react-layout COMMAND dandrum-plugin-editor-bridge-test --react-layout)
    set_tests_properties(tb303-react-layout PROPERTIES LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 45 RUN_SERIAL TRUE)
    add_test(NAME tb303-web-runtime COMMAND dandrum-plugin-editor-bridge-test --web-runtime)
    add_test(NAME sampler-web-runtime COMMAND dandrum-sampler-plugin-host-test --web-runtime)
    set_tests_properties(tb303-web-runtime sampler-web-runtime PROPERTIES
        LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 90 RUN_SERIAL TRUE)

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

add_test(NAME demo-launcher
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_demo_launcher.py)
add_test(NAME demo-launcher-inventory
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_demo_inventory.py)
add_test(NAME demo-launcher-boundaries
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_demo_launcher_boundaries.py)

add_test(NAME mutation-hook
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_mutation_hook.py)

if(NOT DANDRUM_NATIVE_ONLY)
    add_test(NAME shared-instrument-ui COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/SharedInstrumentUiTest.mjs)
    add_test(NAME tb303-page COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/Tb303PageTest.mjs)
    add_test(NAME tb303-react-model COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/tb303/src/model.test.mjs)
    add_test(NAME tb303-react-audition COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/tb303/src/note-audition.test.mjs)
    add_test(NAME native-function-bootstrap COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/NativeFunctionBootstrapTest.mjs)
    add_test(NAME meter-web-transport COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/meter-transport.test.mjs)
    add_test(NAME live-analysis-web-transport COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/live-analysis-transport.test.mjs)
    add_test(NAME live-analysis-web-view COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/live-analysis-view.test.mjs)
    add_test(NAME meter-web-view COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/meter-view.test.mjs)
    add_test(NAME prepared-waveform-web-view COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/prepared-waveform.test.mjs)
    add_test(NAME prepared-waveform-web-transport COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/waveform-transport.test.mjs)
    add_test(NAME prepared-spectral-web-view COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/prepared-spectrum.test.mjs)
    add_test(NAME prepared-spectral-web-transport COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/spectrum-transport.test.mjs)
    add_test(NAME sampler-react-model COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/model.test.mjs)
    add_test(NAME sampler-react-key-map COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/key-map.test.mjs)
    add_test(NAME sampler-react-key-map-interactions COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/key-map-interactions.test.mjs)
    add_test(NAME sampler-react-layer-stack COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/layer-stack.test.mjs)
    add_test(NAME sampler-react-output-buses COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/output-buses.test.mjs)
    add_test(NAME sampler-react-gestures COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/parameter-gesture.test.mjs)
    add_test(NAME knob-parameter-values COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/parameter-value.test.mjs)
    add_test(NAME shared-parameter-controller COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/parameter-controller.test.mjs)
    add_test(NAME knob-snapshot-reconciliation COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/knob-reconciliation.test.mjs)
    add_test(NAME shared-knob-admission COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/host-knob-admission.test.mjs)
    add_test(NAME shared-host-parameters COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/sampler/src/host-parameters.test.mjs)
    add_test(NAME react-plugin-efficiency COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/ReactPluginEfficiencyTest.mjs)
    add_test(NAME prepared-job-polling COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/prepared-polling.test.mjs)
    add_test(NAME prepared-parameter-document COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/web/shared/prepared-parameter-document.test.mjs)
    add_test(NAME design-token-generation COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/DesignTokensTest.mjs)
    add_test(NAME design-tokens-current COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/scripts/generate-ui-tokens.mjs --check)
    add_test(NAME react-asset-packaging COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/ReactAssetsTest.mjs)
    add_test(NAME design-icon-rendering COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/DesignIconsTest.mjs)

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
            dandrum-plugin-live-processor-test
            dandrum-plugin-engine-handoff-test
            dandrum-plugin-knob-parity-test
            dandrum-plugin-waveform-parity-test
            dandrum-plugin-spectral-parity-test
            dandrum-plugin-spectral-job-test
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
