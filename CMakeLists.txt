cmake_minimum_required(VERSION 3.22)

project(Dandrum VERSION 0.1.0 LANGUAGES C CXX)

enable_testing()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
option(DANDRUM_ENABLE_MUTATION_CTEST "Include the long-running cargo-mutants test in CTest" OFF)
option(DANDRUM_ENGINE_ONLY "Build only the Rust engine and independent native consumers without JUCE or browser dependencies" OFF)
option(DANDRUM_CLAP_PROOF "Build the experimental headless CLAP engine proof" OFF)
option(DANDRUM_TRIGGER_CLAP_EDITOR "Build the JUCE-free Trigger CLAP product and stage editor resources" OFF)
option(DANDRUM_TRIGGER_EDITOR_NATIVE_X11 "Enable the real GTK/Wry X11 editor boundary tests" OFF)
set(DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR "" CACHE PATH "Prebuilt Trigger editor bundle to stage beside the CLAP proof")
option(DANDRUM_NATIVE_ONLY "Build the plugin with the native JUCE editor and no browser dependencies" OFF)
option(DANDRUM_TRIGGER_REACT "Embed the full native-backed React sampler shell" OFF)
option(DANDRUM_BUILD_FILTER_UI_SPIKES "Build optional Slint and JIVE filter experiments" OFF)
option(DANDRUM_BUILD_TRIGGER_SLINT "Build the silent Trigger Slint performance preview" OFF)
option(DANDRUM_SLINT_LIBRARY_ONLY "Build only the Slint design library catalog and its tests" OFF)
option(DANDRUM_ADVANCED_SAMPLER_ONLY "Build only the native silent advanced sampler and its tests" OFF)
if(DANDRUM_TRIGGER_EDITOR_NATIVE_X11 AND NOT DANDRUM_TRIGGER_CLAP_EDITOR)
    message(FATAL_ERROR "DANDRUM_TRIGGER_EDITOR_NATIVE_X11 requires DANDRUM_TRIGGER_CLAP_EDITOR")
endif()
if(DANDRUM_CLAP_PROOF)
    if(NOT DANDRUM_ENGINE_ONLY)
        message(FATAL_ERROR "DANDRUM_CLAP_PROOF requires DANDRUM_ENGINE_ONLY")
    endif()
    if(NOT CMAKE_SYSTEM_NAME STREQUAL "Linux")
        message(FATAL_ERROR "The experimental CLAP packaging proof is currently verified only on Linux")
    endif()
endif()
if(DANDRUM_TRIGGER_CLAP_EDITOR)
    if(NOT DANDRUM_CLAP_PROOF)
        message(FATAL_ERROR "DANDRUM_TRIGGER_CLAP_EDITOR requires DANDRUM_CLAP_PROOF")
    endif()
    if(NOT IS_DIRECTORY "${DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR}")
        message(FATAL_ERROR "DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR must name a prebuilt Trigger editor bundle")
    endif()
    foreach(DANDRUM_TRIGGER_EDITOR_FILE IN ITEMS index.html app.js app.css)
        if(NOT EXISTS "${DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR}/${DANDRUM_TRIGGER_EDITOR_FILE}")
            message(FATAL_ERROR
                "DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR is missing ${DANDRUM_TRIGGER_EDITOR_FILE}")
        endif()
    endforeach()
    file(GLOB_RECURSE DANDRUM_TRIGGER_EDITOR_BUNDLE_FILES
        CONFIGURE_DEPENDS LIST_DIRECTORIES FALSE
        "${DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR}/*")
endif()
if(DANDRUM_ENGINE_ONLY)
    foreach(DANDRUM_PRODUCT_OPTION IN ITEMS
            DANDRUM_NATIVE_ONLY DANDRUM_TRIGGER_REACT
            DANDRUM_BUILD_FILTER_UI_SPIKES DANDRUM_BUILD_TRIGGER_SLINT
            DANDRUM_SLINT_LIBRARY_ONLY DANDRUM_ADVANCED_SAMPLER_ONLY)
        if(${DANDRUM_PRODUCT_OPTION})
            message(FATAL_ERROR "DANDRUM_ENGINE_ONLY cannot be combined with ${DANDRUM_PRODUCT_OPTION}")
        endif()
    endforeach()
endif()
if(DANDRUM_ADVANCED_SAMPLER_ONLY)
    add_executable(dandrum-advanced-sampler)
    add_subdirectory(ui/advanced-sampler)
    return()
endif()
if(DANDRUM_SLINT_LIBRARY_ONLY)
    # Keep the reusable UI build independent of JUCE, Rust, Node, and the plugin.
    add_executable(dandrum-slint-catalog)
    add_subdirectory(ui/slint)
    return()
endif()
if(NOT DANDRUM_ENGINE_ONLY)
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
endif()

set(RUST_ENGINE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/rust-engine)
set(RUST_ENGINE_LIB ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_engine.a)
set(RUST_SESSION_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/instrument-session)
set(RUST_SESSION_LIB ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_instrument_session.a)
set(RUST_TRIGGER_HOST_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/trigger-host)
set(RUST_TRIGGER_HOST_LIB ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_trigger_host.rlib)
set(RUST_TRIGGER_EDITOR_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/trigger-editor)
set(RUST_TRIGGER_EDITOR_LIB ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_trigger_editor.rlib)
set(CARGO_BIN $ENV{HOME}/.cargo/bin/cargo)
set(RUST_ENGINE_MUTATION_EXCLUDE_ARGS
    --exclude src/bin/*.rs
    --exclude src/cli.rs)

file(GLOB_RECURSE RUST_ENGINE_SOURCES CONFIGURE_DEPENDS ${RUST_ENGINE_DIR}/src/*.rs)
file(GLOB_RECURSE RUST_SESSION_SOURCES CONFIGURE_DEPENDS ${RUST_SESSION_DIR}/src/*.rs)
file(GLOB_RECURSE RUST_TRIGGER_HOST_SOURCES CONFIGURE_DEPENDS ${RUST_TRIGGER_HOST_DIR}/src/*.rs)
file(GLOB_RECURSE RUST_TRIGGER_EDITOR_SOURCES CONFIGURE_DEPENDS ${RUST_TRIGGER_EDITOR_DIR}/src/*.rs)

add_custom_command(
    OUTPUT ${RUST_ENGINE_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target ${CARGO_BIN} build --release
    WORKING_DIRECTORY ${RUST_ENGINE_DIR}
    DEPENDS
        ${RUST_ENGINE_DIR}/Cargo.toml
        ${RUST_ENGINE_SOURCES}
    COMMENT "Building Rust audio engine")

add_custom_target(dandrum-engine-rust DEPENDS ${RUST_ENGINE_LIB})

add_custom_command(
    OUTPUT ${RUST_SESSION_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
        ${CARGO_BIN} build --release --locked --manifest-path ${RUST_SESSION_DIR}/Cargo.toml
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    DEPENDS
        ${RUST_SESSION_DIR}/Cargo.toml
        ${RUST_SESSION_DIR}/Cargo.lock
        ${RUST_SESSION_SOURCES}
        ${RUST_ENGINE_DIR}/Cargo.toml
        ${RUST_ENGINE_SOURCES}
    COMMENT "Building Rust instrument session facade")

add_custom_target(dandrum-instrument-session-rust DEPENDS ${RUST_SESSION_LIB})

if(DANDRUM_TRIGGER_CLAP_EDITOR)
    add_custom_command(
        OUTPUT ${RUST_TRIGGER_HOST_LIB}
        COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
            ${CARGO_BIN} build --release --locked --jobs 2
            --manifest-path ${RUST_TRIGGER_HOST_DIR}/Cargo.toml --lib
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        DEPENDS
            ${RUST_TRIGGER_HOST_DIR}/Cargo.toml
            ${RUST_TRIGGER_HOST_DIR}/Cargo.lock
            ${RUST_TRIGGER_HOST_SOURCES}
            ${RUST_SESSION_DIR}/Cargo.toml
            ${RUST_SESSION_DIR}/Cargo.lock
            ${RUST_SESSION_SOURCES}
            ${RUST_ENGINE_DIR}/Cargo.toml
            ${RUST_ENGINE_DIR}/Cargo.lock
            ${RUST_ENGINE_SOURCES}
        COMMENT "Building the format-neutral Trigger host scaffold")
    add_custom_target(dandrum-trigger-host-rust DEPENDS ${RUST_TRIGGER_HOST_LIB})
    add_test(NAME trigger-host-tests
        COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
            ${CARGO_BIN} test --locked --jobs 2
            --manifest-path ${RUST_TRIGGER_HOST_DIR}/Cargo.toml)
    set_tests_properties(trigger-host-tests PROPERTIES LABELS "trigger-host-fast" TIMEOUT 1200)
    add_custom_command(
        OUTPUT ${RUST_TRIGGER_EDITOR_LIB}
        COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
            ${CARGO_BIN} build --release --locked --jobs 2
            --manifest-path ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml --lib
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        DEPENDS
            ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml
            ${RUST_TRIGGER_EDITOR_DIR}/Cargo.lock
            ${RUST_TRIGGER_EDITOR_SOURCES}
        COMMENT "Building the concrete Trigger editor asset boundary")
    add_custom_target(dandrum-trigger-editor-rust DEPENDS ${RUST_TRIGGER_EDITOR_LIB})
    add_test(NAME trigger-editor-tests
        COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
            ${CARGO_BIN} test --locked --jobs 2
            --manifest-path ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml --lib)
    set_tests_properties(trigger-editor-tests PROPERTIES LABELS "trigger-editor-fast" TIMEOUT 60)
endif()

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

add_test(
    NAME instrument-session-tests
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
        ${CARGO_BIN} test --locked --jobs 2
        --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/src/instrument-session/Cargo.toml)
set_tests_properties(instrument-session-tests PROPERTIES LABELS "session-fast" TIMEOUT 1200)

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

add_library(dandrum_instrument_session STATIC IMPORTED GLOBAL)
set_target_properties(dandrum_instrument_session PROPERTIES IMPORTED_LOCATION ${RUST_SESSION_LIB})
add_dependencies(dandrum_instrument_session dandrum-instrument-session-rust)

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

add_executable(dandrum-filter-spike-engine-test tests/cpp/FilterSpikeEngineTest.cpp)
target_include_directories(dandrum-filter-spike-engine-test PRIVATE src/juce-wrapper)
target_compile_definitions(dandrum-filter-spike-engine-test PRIVATE DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}")
target_link_libraries(dandrum-filter-spike-engine-test PRIVATE dandrum_engine dl pthread m)
add_test(NAME filter-spike-engine COMMAND dandrum-filter-spike-engine-test)

add_test(NAME engine-only-build
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3
        ${CMAKE_CURRENT_SOURCE_DIR}/tests/python/test_engine_only_build.py EngineOnlyBuildTest)
set_tests_properties(engine-only-build PROPERTIES LABELS "build-boundary" RUN_SERIAL TRUE TIMEOUT 1200)

if(DANDRUM_CLAP_PROOF)
    add_test(NAME trigger-clap-native-fixture-driver
        COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/tests/python/test_trigger_native_fixture.py)
    set_tests_properties(trigger-clap-native-fixture-driver PROPERTIES
        LABELS "trigger-clap-boundary;fast" TIMEOUT 30)
    set(DANDRUM_CLAP_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/clap-adapter)
    set(DANDRUM_CLAP_LIBRARY ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_clap_adapter.so)
    set(DANDRUM_CLAP_ARTIFACT ${CMAKE_CURRENT_BINARY_DIR}/clap/DandrumHeadlessProof.clap)
    file(GLOB_RECURSE DANDRUM_CLAP_SOURCES CONFIGURE_DEPENDS
        ${DANDRUM_CLAP_DIR}/src/*.rs
        ${DANDRUM_CLAP_DIR}/fixtures/*.yaml)
    add_custom_command(
        OUTPUT ${DANDRUM_CLAP_ARTIFACT}
        COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
            ${CARGO_BIN} build --locked --release --jobs 2 --manifest-path ${DANDRUM_CLAP_DIR}/Cargo.toml --lib
        COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_CURRENT_BINARY_DIR}/clap
        COMMAND ${CMAKE_COMMAND} -E copy_if_different ${DANDRUM_CLAP_LIBRARY} ${DANDRUM_CLAP_ARTIFACT}
        DEPENDS ${DANDRUM_CLAP_SOURCES} ${RUST_ENGINE_SOURCES}
            ${DANDRUM_CLAP_DIR}/Cargo.toml ${DANDRUM_CLAP_DIR}/Cargo.lock
            ${RUST_ENGINE_DIR}/Cargo.toml ${RUST_ENGINE_DIR}/Cargo.lock
            ${RUST_ENGINE_DIR}/module-library/seed-1.0.0.zip
            ${CMAKE_CURRENT_SOURCE_DIR}/schema/patch.schema.yaml
            ${CMAKE_CURRENT_SOURCE_DIR}/examples/patches/tb303-acid.yaml
        COMMENT "Building the experimental JUCE-free CLAP proof"
        VERBATIM)
    add_custom_target(dandrum-clap-proof DEPENDS ${DANDRUM_CLAP_ARTIFACT})
    if(DANDRUM_TRIGGER_CLAP_EDITOR)
        set(DANDRUM_TRIGGER_CLAP_ARTIFACT ${CMAKE_CURRENT_BINARY_DIR}/clap/DandrumTrigger.clap)
        add_custom_command(
            OUTPUT ${DANDRUM_TRIGGER_CLAP_ARTIFACT}
            COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
                ${CARGO_BIN} build --locked --release --jobs 2
                --manifest-path ${DANDRUM_CLAP_DIR}/Cargo.toml --features trigger --lib
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${DANDRUM_CLAP_LIBRARY} ${DANDRUM_TRIGGER_CLAP_ARTIFACT}
            # Both features reuse the warm Cargo directory. The copied default
            # artifact must finish before this feature can overwrite its library.
            DEPENDS ${DANDRUM_CLAP_ARTIFACT} ${DANDRUM_CLAP_SOURCES}
                ${RUST_ENGINE_SOURCES} ${RUST_SESSION_SOURCES} ${RUST_TRIGGER_HOST_SOURCES}
                ${DANDRUM_CLAP_DIR}/Cargo.toml ${DANDRUM_CLAP_DIR}/Cargo.lock
                ${RUST_SESSION_DIR}/Cargo.toml ${RUST_SESSION_DIR}/Cargo.lock
                ${RUST_TRIGGER_HOST_DIR}/Cargo.toml ${RUST_TRIGGER_HOST_DIR}/Cargo.lock
            COMMENT "Building the session-backed JUCE-free Trigger CLAP product"
            VERBATIM)
        add_custom_target(dandrum-trigger-clap-product DEPENDS ${DANDRUM_TRIGGER_CLAP_ARTIFACT})
        add_test(NAME trigger-clap-dynamic-host
            COMMAND ${CMAKE_COMMAND} -E env
                CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
                DANDRUM_TRIGGER_CLAP_PLUGIN_PATH=${DANDRUM_TRIGGER_CLAP_ARTIFACT}
                ${CARGO_BIN} test --locked --jobs 2
                --manifest-path ${DANDRUM_CLAP_DIR}/Cargo.toml
                --features trigger --test trigger_dynamic)
        set_tests_properties(trigger-clap-dynamic-host PROPERTIES
            LABELS "trigger-clap-boundary" TIMEOUT 1200)
        set(DANDRUM_TRIGGER_RESOURCE_DIR ${CMAKE_CURRENT_BINARY_DIR}/clap/DandrumTrigger.resources)
        set(DANDRUM_TRIGGER_RESOURCE_STAMP ${DANDRUM_TRIGGER_RESOURCE_DIR}/.staged)
        add_custom_command(
            OUTPUT ${DANDRUM_TRIGGER_RESOURCE_STAMP}
            COMMAND ${CMAKE_COMMAND} -E rm -rf ${DANDRUM_TRIGGER_RESOURCE_DIR}
            COMMAND ${CMAKE_COMMAND} -E make_directory
                ${DANDRUM_TRIGGER_RESOURCE_DIR}/editor
                ${DANDRUM_TRIGGER_RESOURCE_DIR}/instrument/assets
            COMMAND ${CMAKE_COMMAND} -E copy_directory
                ${DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR}
                ${DANDRUM_TRIGGER_RESOURCE_DIR}/editor
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_SOURCE_DIR}/examples/patches/advanced-drum-kit.yaml
                ${DANDRUM_TRIGGER_RESOURCE_DIR}/instrument/advanced-drum-kit.yaml
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_SOURCE_DIR}/examples/patches/assets/advanced-drums.wav
                ${DANDRUM_TRIGGER_RESOURCE_DIR}/instrument/assets/advanced-drums.wav
            COMMAND ${CMAKE_COMMAND} -E touch ${DANDRUM_TRIGGER_RESOURCE_STAMP}
            DEPENDS
                ${DANDRUM_TRIGGER_EDITOR_BUNDLE_FILES}
                ${CMAKE_CURRENT_SOURCE_DIR}/examples/patches/advanced-drum-kit.yaml
                ${CMAKE_CURRENT_SOURCE_DIR}/examples/patches/assets/advanced-drums.wav
            COMMENT "Staging copied Trigger editor and instrument resources"
            VERBATIM)
        add_custom_target(dandrum-trigger-resources DEPENDS ${DANDRUM_TRIGGER_RESOURCE_STAMP})
        add_dependencies(dandrum-trigger-clap-product dandrum-trigger-resources)
        # This lane requires the complete production bundle, including local
        # fonts/notices. The Python source-free build fixture deliberately uses
        # a tiny build-only bundle and separately checks this test's registration.
        add_test(NAME trigger-editor-packaged-assets
            COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
                DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=${DANDRUM_TRIGGER_RESOURCE_DIR}/editor
                ${CARGO_BIN} test --locked --jobs 2
                --manifest-path ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml
                --test packaged_assets -- --ignored)
        set_tests_properties(trigger-editor-packaged-assets PROPERTIES
            LABELS "trigger-editor-boundary" TIMEOUT 60)
        if(DANDRUM_TRIGGER_EDITOR_NATIVE_X11)
            # Prepare the native test binary before its bounded runtime checks.
            add_test(NAME trigger-editor-native-prepare
                COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
                    ${CARGO_BIN} test --release --locked --jobs 2
                    --manifest-path ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml
                    --features native-x11 --lib --no-run)
            set_tests_properties(trigger-editor-native-prepare PROPERTIES
                LABELS "trigger-editor-boundary;native-x11"
                FIXTURES_SETUP trigger-editor-native-compiled
                RUN_SERIAL TRUE TIMEOUT 1200)
            add_test(NAME trigger-editor-native-x11
                COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
                    GDK_BACKEND=x11
                    DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=${DANDRUM_TRIGGER_RESOURCE_DIR}/editor
                    ${CARGO_BIN} test --release --locked --jobs 2
                    --manifest-path ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml
                    --features native-x11 --lib
                    native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent
                    -- --ignored --exact --nocapture --test-threads=1)
            add_test(NAME trigger-editor-native-handler-lifetime
                COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
                    GDK_BACKEND=x11
                    DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=${DANDRUM_TRIGGER_RESOURCE_DIR}/editor
                    ${CARGO_BIN} test --release --locked --jobs 2
                    --manifest-path ${RUST_TRIGGER_EDITOR_DIR}/Cargo.toml
                    --features native-x11 --lib
                    native_x11::tests::destroy_releases_application_handlers_while_framework_context_remains
                    -- --ignored --exact --nocapture --test-threads=1)
            foreach(native_test IN ITEMS trigger-editor-native-x11 trigger-editor-native-handler-lifetime)
                set_tests_properties(${native_test} PROPERTIES
                    LABELS "trigger-editor-boundary;native-x11"
                    FIXTURES_REQUIRED trigger-editor-native-compiled
                    RUN_SERIAL TRUE TIMEOUT 60
                    # libtest succeeds when an exact filter matches no test.
                    FAIL_REGULAR_EXPRESSION "running 0 tests")
            endforeach()
            # Private developer lifecycle fixture, separate from the headless
            # product. Select Cargo's reported artifacts, never convenience paths.
            set(DANDRUM_TRIGGER_NATIVE_FIXTURE ${CMAKE_CURRENT_BINARY_DIR}/trigger-clap-native-fixture)
            add_test(NAME trigger-clap-native-prepare
                COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/scripts/prepare-trigger-clap-native.py
                    --cargo ${CARGO_BIN}
                    --manifest ${DANDRUM_CLAP_DIR}/Cargo.toml
                    --target-dir ${CMAKE_CURRENT_BINARY_DIR}/rust-target
                    --resource-dir ${DANDRUM_TRIGGER_RESOURCE_DIR}
                    --output-dir ${DANDRUM_TRIGGER_NATIVE_FIXTURE})
            set_tests_properties(trigger-clap-native-prepare PROPERTIES
                LABELS "trigger-clap-boundary;native-x11"
                FIXTURES_SETUP trigger-clap-native-compiled
                RUN_SERIAL TRUE TIMEOUT 1200)
            set(trigger_native_names extension lifecycle configuration loader-identity failed-parent)
            set(trigger_native_cases
                copied_trigger_editor_exposes_the_actual_clap_gui_extension
                copied_native_editor_lifecycle_initializes_the_real_react_session_and_preserves_audio
                copied_editor_scan_and_configuration_only_do_not_make_code_resident
                copied_editor_rejects_wrong_loaded_entry_identity_before_gtk_without_state_or_audio_loss
                copied_editor_failed_native_parent_attempt_retains_only_code_and_preserves_audio)
            foreach(native_name native_case IN ZIP_LISTS trigger_native_names trigger_native_cases)
                add_test(NAME trigger-clap-native-${native_name}
                    COMMAND ${CMAKE_COMMAND} -E env GDK_BACKEND=x11
                        DANDRUM_TRIGGER_GUI_PLUGIN_PATH=${DANDRUM_TRIGGER_NATIVE_FIXTURE}/DandrumTrigger.clap
                        ${DANDRUM_TRIGGER_NATIVE_FIXTURE}/trigger-gui-test
                        ${native_case} --ignored --exact --nocapture --test-threads=1)
                set_tests_properties(trigger-clap-native-${native_name} PROPERTIES
                    LABELS "trigger-clap-boundary;native-x11"
                    FIXTURES_REQUIRED trigger-clap-native-compiled
                    RUN_SERIAL TRUE TIMEOUT 60
                    FAIL_REGULAR_EXPRESSION "running 0 tests")
            endforeach()
            # Public WebKit evaluation lives in a separate test executable so
            # the G1 loader fixture keeps its independent no-WebKit-host oracle.
            set(DANDRUM_TRIGGER_PARAMETERS_FIXTURE ${CMAKE_CURRENT_BINARY_DIR}/trigger-clap-parameters-fixture)
            add_test(NAME trigger-clap-native-parameters-prepare
                COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/scripts/prepare-trigger-clap-native.py
                    --cargo ${CARGO_BIN}
                    --manifest ${DANDRUM_CLAP_DIR}/Cargo.toml
                    --target-dir ${CMAKE_CURRENT_BINARY_DIR}/rust-target
                    --resource-dir ${DANDRUM_TRIGGER_RESOURCE_DIR}
                    --output-dir ${DANDRUM_TRIGGER_PARAMETERS_FIXTURE}
                    --test-target trigger_parameters)
            set_tests_properties(trigger-clap-native-parameters-prepare PROPERTIES
                LABELS "trigger-clap-boundary;native-x11"
                FIXTURES_SETUP trigger-clap-parameters-compiled
                RUN_SERIAL TRUE TIMEOUT 1200)
            set(trigger_parameter_names
                state flush ownership
                timer-required timer-refused timer-delivery timer-rollback
                source-credits feedback-undo callback-audit invalid-stopped-flush structural-session
                note-hold note-heartbeat note-expiry note-destroy creation-failure
                continuous-audio continuous-audio-audit two-instances)
            set(trigger_parameter_cases
                copied_native_parameter_state_is_an_authenticated_numeric_dto
                copied_native_parameter_flush_delivers_host_accepted_gestures
                copied_native_automation_survives_rejected_output_activation_failures_and_audio_return
                copied_native_gui_requires_host_timer
                copied_native_gui_rejects_refused_host_timer
                copied_native_host_timer_delivers_a_later_rendered_parameter_edit
                copied_native_timer_rollback_retains_disabled_cleanup_and_retries
                copied_native_view_independent_source_collects_accepted_automation_credits
                copied_native_host_feedback_updates_rendered_control_without_polling_or_undo_pollution
                copied_native_parameter_callbacks_allocate_and_destroy_nothing_while_gui_services
                copied_native_invalid_stopped_flush_preserves_uncollected_automation_ownership
                structural_native::copied_native_mapping_survives_lost_reply_and_view_loss
                note_native::copied_native_gated_sample_outlasts_short_fixture_and_releases
                note_native::copied_native_heartbeat_renews_held_note_past_original_deadline
                note_native::copied_native_live_attachment_expires_renewed_note_and_preserves_host_voice
                note_native::copied_native_destroy_releases_held_activity_once_and_reconnects
                creation_failure_native::copied_native_ipc_registration_failure_cleans_up_and_retries_preserving_state
                note_native::copied_native_continuous_audio_survives_view_replacement
                note_native::copied_native_continuous_audio_callbacks_allocate_and_deallocate_nothing
                instances_native::copied_native_two_instances_keep_edits_audio_and_editors_independent)
            foreach(parameter_name parameter_case IN ZIP_LISTS trigger_parameter_names trigger_parameter_cases)
                add_test(NAME trigger-clap-native-parameter-${parameter_name}
                    COMMAND ${CMAKE_COMMAND} -E env GDK_BACKEND=x11
                        DANDRUM_TRIGGER_GUI_PLUGIN_PATH=${DANDRUM_TRIGGER_PARAMETERS_FIXTURE}/DandrumTrigger.clap
                        ${DANDRUM_TRIGGER_PARAMETERS_FIXTURE}/trigger-parameters-test
                        ${parameter_case} --ignored --exact --nocapture --test-threads=1)
                set_tests_properties(trigger-clap-native-parameter-${parameter_name} PROPERTIES
                    LABELS "trigger-clap-boundary;native-x11"
                    FIXTURES_REQUIRED trigger-clap-parameters-compiled
                    RUN_SERIAL TRUE TIMEOUT 60
                    FAIL_REGULAR_EXPRESSION "running 0 tests")
            endforeach()
        endif()
        add_custom_target(dandrum-trigger-clap-editor)
        add_dependencies(dandrum-trigger-clap-editor
            dandrum-trigger-clap-product dandrum-trigger-host-rust dandrum-trigger-editor-rust dandrum-trigger-resources)
    endif()
    add_test(NAME clap-dynamic-host
        COMMAND ${CMAKE_COMMAND} -E env
            CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target
            DANDRUM_CLAP_PLUGIN_PATH=${DANDRUM_CLAP_ARTIFACT}
            ${CARGO_BIN} test --locked --jobs 2 --manifest-path ${DANDRUM_CLAP_DIR}/Cargo.toml
                --test clap_dynamic --test public_engine_baseline)
    set_tests_properties(clap-dynamic-host PROPERTIES TIMEOUT 1200)
    add_test(NAME clap-build-boundary
        COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/tests/python/test_engine_only_build.py ClapProofBuildTest)
    set_tests_properties(clap-build-boundary PROPERTIES
        LABELS "build-boundary" RUN_SERIAL TRUE TIMEOUT 1200)
endif()

if(DANDRUM_ENGINE_ONLY)
    return()
endif()

add_subdirectory(third_party/JUCE)

add_test(NAME engine-only-comparison-build
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3
        ${CMAKE_CURRENT_SOURCE_DIR}/tests/python/test_engine_only_build.py ComparisonBuildTest)
set_tests_properties(engine-only-comparison-build PROPERTIES LABELS "build-boundary" RUN_SERIAL TRUE TIMEOUT 600)

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
    if(DANDRUM_TRIGGER_REACT)
        file(GLOB_RECURSE trigger_react_files CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/web/trigger/dist-native/*")
        if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/web/trigger/dist-native/index.html")
            message(FATAL_ERROR "Run npm ci --prefix web/trigger and npm run build:native --prefix web/trigger before configuring the integrated sampler")
        endif()
        juce_add_binary_data(dandrum-trigger-react-resources
            NAMESPACE TriggerReactBinaryData HEADER_NAME TriggerReactBinaryData.h
            SOURCES ${trigger_react_files})
    endif()
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
if(DANDRUM_TRIGGER_REACT AND NOT DANDRUM_NATIVE_ONLY)
    target_link_libraries(dandrum-plugin PRIVATE dandrum-trigger-react-resources)
    target_compile_definitions(dandrum-plugin PRIVATE DANDRUM_EMBED_TRIGGER_REACT=1)
endif()
target_compile_definitions(dandrum-plugin PRIVATE
    DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}"
    DANDRUM_EMBED_TB303_ASSETS=1
    JUCE_USE_CURL=0
    JUCE_WEB_BROWSER=$<BOOL:${DANDRUM_BROWSER_ENABLED}>
    JUCE_USE_WIN_WEBVIEW2=$<BOOL:${DANDRUM_BROWSER_ENABLED}>)
target_link_libraries(dandrum-plugin
    PRIVATE dandrum_instrument_session dandrum-tb303-instrument-resources
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
    PRIVATE dandrum_instrument_session dandrum-sampler-resources dandrum-plugin-ui-common ${DANDRUM_EDITOR_ADAPTER}
        juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils
    PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
if(NOT DANDRUM_NATIVE_ONLY)
    target_link_libraries(dandrum-sampler-plugin PRIVATE dandrum-sampler-react-resources)
    target_compile_definitions(dandrum-sampler-plugin PRIVATE DANDRUM_EMBED_SAMPLER_REACT=1)
    if(DANDRUM_TRIGGER_REACT)
        target_link_libraries(dandrum-sampler-plugin PRIVATE dandrum-trigger-react-resources)
        target_compile_definitions(dandrum-sampler-plugin PRIVATE DANDRUM_EMBED_TRIGGER_REACT=1)
    endif()
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
        PRIVATE dandrum-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors
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
            dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors
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
        dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors
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
    dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors
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
    dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_utils juce::juce_dsp
    juce::juce_recommended_config_flags juce::juce_recommended_warning_flags)
if(NOT DANDRUM_NATIVE_ONLY)
    target_link_libraries(dandrum-plugin-live-processor-test PRIVATE juce::juce_gui_extra)
endif()
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    target_link_options(dandrum-plugin-live-processor-test PRIVATE
        "LINKER:--wrap=pthread_mutex_lock" "LINKER:--wrap=_ZNSt6thread4joinEv"
        "LINKER:--undefined=_ZNK4juce3dsp3FFT36performFrequencyOnlyForwardTransformEPfb"
        "LINKER:--wrap=_ZNK4juce3dsp3FFT36performFrequencyOnlyForwardTransformEPfb"
        "LINKER:--wrap=dandrum_kernel_destroy" "LINKER:--wrap=dandrum_kernel_prepare_file")
endif()
add_test(NAME cxx-plugin-live-processor COMMAND dandrum-plugin-live-processor-test)
set_tests_properties(cxx-plugin-live-processor PROPERTIES TIMEOUT 15)

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    add_executable(dandrum-plugin-engine-handoff-test tests/cpp/PluginEngineHandoffTest.cpp)
    target_include_directories(dandrum-plugin-engine-handoff-test PRIVATE src/juce-wrapper src/juce-plugin)
    # GCC 15 hides the thread state used by this GNU ABI failure-injection seam.
    # Expose it only in the test target, as libstdc++ does for its implementation.
    target_compile_definitions(dandrum-plugin-engine-handoff-test PRIVATE
        _GLIBCXX_THREAD_IMPL
        DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}" JUCE_USE_CURL=0
        JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
        JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
    target_link_libraries(dandrum-plugin-engine-handoff-test PRIVATE
        dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_utils
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

    add_executable(dandrum-plugin-instrument-session-test tests/cpp/PluginInstrumentSessionTest.cpp)
    target_include_directories(dandrum-plugin-instrument-session-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-instrument-session-test PRIVATE
        DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}" JUCE_USE_CURL=0
        JUCE_WEB_BROWSER=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>
        JUCE_USE_WIN_WEBVIEW2=$<NOT:$<BOOL:${DANDRUM_NATIVE_ONLY}>>)
    target_link_libraries(dandrum-plugin-instrument-session-test PRIVATE
        dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_utils
        juce::juce_recommended_config_flags juce::juce_recommended_warning_flags)
    if(NOT DANDRUM_NATIVE_ONLY)
        target_link_libraries(dandrum-plugin-instrument-session-test PRIVATE juce::juce_gui_extra)
    endif()
    target_link_options(dandrum-plugin-instrument-session-test PRIVATE
        "LINKER:--wrap=pthread_mutex_lock"
        "LINKER:--wrap=dandrum_kernel_render" "LINKER:--wrap=dandrum_kernel_destroy"
        "LINKER:--wrap=dandrum_instrument_session_prepare_job")
    add_test(NAME cxx-plugin-instrument-session COMMAND dandrum-plugin-instrument-session-test)
    add_test(NAME native-session-link-graph
        COMMAND ${CMAKE_COMMAND}
            -DNM=${CMAKE_NM}
            -DSESSION_LIB=${RUST_SESSION_LIB}
            -DCONSUMER=$<TARGET_FILE:dandrum-plugin-instrument-session-test>
            -DLINK_FILE=${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles/dandrum-plugin-instrument-session-test.dir/link.txt
            -DSTANDALONE=$<TARGET_FILE:dandrum-sampler-plugin_Standalone>
            -DSTANDALONE_LINK_FILE=${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles/dandrum-sampler-plugin_Standalone.dir/link.txt
            -DVST3=$<TARGET_FILE:dandrum-sampler-plugin_VST3>
            -DVST3_LINK_FILE=${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles/dandrum-sampler-plugin_VST3.dir/link.txt
            -P ${CMAKE_CURRENT_SOURCE_DIR}/tests/cmake/NativeSessionLinkGraph.cmake)
    set_tests_properties(cxx-plugin-instrument-session native-session-link-graph PROPERTIES TIMEOUT 15)
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
    dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors
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
    dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors
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
        PRIVATE dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-sampler-plugin-host COMMAND dandrum-sampler-plugin-host-test)
    add_test(NAME cxx-output-bus-metadata COMMAND dandrum-sampler-plugin-host-test --output-bus-metadata)

    add_executable(dandrum-plugin-construction-test tests/cpp/PluginConstructionTest.cpp)
    target_include_directories(dandrum-plugin-construction-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-construction-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-construction-test
        PRIVATE dandrum-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-construction COMMAND dandrum-plugin-construction-test)

    add_executable(dandrum-plugin-host-parameter-surface-test tests/cpp/PluginHostParameterSurfaceTest.cpp)
    target_include_directories(dandrum-plugin-host-parameter-surface-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-host-parameter-surface-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-host-parameter-surface-test
        PRIVATE dandrum-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-host-parameter-surface COMMAND dandrum-plugin-host-parameter-surface-test)

    add_executable(dandrum-plugin-editor-bridge-test tests/cpp/PluginEditorBridgeTest.cpp)
    target_include_directories(dandrum-plugin-editor-bridge-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-editor-bridge-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-editor-bridge-test
        PRIVATE dandrum-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-editor-bridge COMMAND dandrum-plugin-editor-bridge-test)
    add_executable(dandrum-trigger-native-bridge-test tests/cpp/PluginTriggerNativeBridgeTest.cpp)
    target_include_directories(dandrum-trigger-native-bridge-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-trigger-native-bridge-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-trigger-native-bridge-test PRIVATE dandrum-plugin dandrum_instrument_session
        juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-trigger-native-bridge COMMAND dandrum-trigger-native-bridge-test)
    if(DANDRUM_TRIGGER_REACT)
        add_test(NAME trigger-web-runtime COMMAND dandrum-trigger-native-bridge-test --web-runtime)
        set_tests_properties(trigger-web-runtime PROPERTIES
            LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 120 RUN_SERIAL TRUE)
        add_executable(dandrum-standalone-chrome-test tests/cpp/PluginStandaloneChromeTest.cpp)
        target_include_directories(dandrum-standalone-chrome-test PRIVATE src/juce-wrapper src/juce-plugin)
        target_compile_definitions(dandrum-standalone-chrome-test PRIVATE
            JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
        target_link_libraries(dandrum-standalone-chrome-test PRIVATE
            dandrum-sampler-plugin dandrum_instrument_session juce::juce_audio_utils juce::juce_gui_extra
            juce::juce_recommended_config_flags juce::juce_recommended_warning_flags)
        add_test(NAME cxx-standalone-chrome COMMAND dandrum-standalone-chrome-test)
        set_tests_properties(cxx-standalone-chrome PROPERTIES
            LABELS "runtime" SKIP_RETURN_CODE 77 TIMEOUT 45 RUN_SERIAL TRUE)
    endif()
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
        PRIVATE dandrum-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
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

add_test(NAME slint-headless-launch
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover
            -s ${CMAKE_CURRENT_SOURCE_DIR}/tests/python
            -p test_slint_headless.py)

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    add_test(NAME slint-compiler-build-environment
        COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1 python3
                ${CMAKE_CURRENT_SOURCE_DIR}/tests/python/test_slint_build_environment.py)
endif()

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
    add_test(NAME trigger-react-shell COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/TriggerReactShellTest.mjs)
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
    add_test(NAME design-token-generation COMMAND ${DANDRUM_NODE_EXECUTABLE} --test
        ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/DesignTokensTest.mjs
        ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/SlintDesignTokensTest.mjs)
    add_test(NAME slint-library-inventory COMMAND ${DANDRUM_NODE_EXECUTABLE} --test
        ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/SlintLibraryInventoryTest.mjs)
    add_test(NAME design-tokens-current COMMAND ${DANDRUM_NODE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/scripts/generate-ui-tokens.mjs --check)
    add_test(NAME react-asset-packaging COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/ReactAssetsTest.mjs)
    add_test(NAME design-icon-rendering COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/DesignIconsTest.mjs)
    add_test(NAME trigger-native-ui COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/TriggerNativeUiTest.mjs)
    add_test(NAME trigger-native-adapter COMMAND ${DANDRUM_NODE_EXECUTABLE} --test ${CMAKE_CURRENT_SOURCE_DIR}/tests/js/TriggerNativeAdapterTest.mjs)

    add_executable(dandrum-plugin-file-watch-reload-test tests/cpp/PluginFileWatchReloadTest.cpp)
    target_include_directories(dandrum-plugin-file-watch-reload-test PRIVATE src/juce-wrapper src/juce-plugin)
    target_compile_definitions(dandrum-plugin-file-watch-reload-test PRIVATE JUCE_USE_CURL=0 JUCE_WEB_BROWSER=1 JUCE_USE_WIN_WEBVIEW2=1)
    target_link_libraries(dandrum-plugin-file-watch-reload-test
        PRIVATE dandrum-plugin dandrum_instrument_session juce::juce_audio_basics juce::juce_audio_processors juce::juce_audio_utils juce::juce_gui_extra
        PUBLIC juce::juce_recommended_config_flags juce::juce_recommended_lto_flags juce::juce_recommended_warning_flags)
    add_test(NAME cxx-plugin-file-watch-reload COMMAND dandrum-plugin-file-watch-reload-test)
endif()

# These plain test executables compile JUCE's web-browser module themselves.
# Unlike juce_add_plugin(), add_executable() does not automatically attach the
# Linux WebKit/GTK usage requirements requested by NEEDS_WEB_BROWSER.
if(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND NOT DANDRUM_NATIVE_ONLY)
    if(TARGET dandrum-standalone-chrome-test)
        # Window chrome checks do not need PulseAudio or a hardware stream.
        # Avoid blocking on device enumeration when WSLg's audio server stalls.
        set_tests_properties(cxx-standalone-chrome PROPERTIES
            ENVIRONMENT "PULSE_SERVER=unix:${CMAKE_CURRENT_BINARY_DIR}/standalone-chrome-no-pulse.sock")
        target_include_directories(dandrum-standalone-chrome-test PRIVATE
            ${CMAKE_CURRENT_BINARY_DIR}/dandrum-sampler-plugin_artefacts/JuceLibraryCode
            ${CMAKE_CURRENT_BINARY_DIR}/dandrum-plugin_artefacts/JuceLibraryCode)
        target_link_libraries(dandrum-standalone-chrome-test PRIVATE juce::pkgconfig_JUCE_BROWSER_LINUX_DEPS)
        juce_link_with_embedded_linux_subprocess(dandrum-standalone-chrome-test)
    endif()
    foreach(plugin_test_target
            dandrum-trigger-native-bridge-test
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
        juce_link_with_embedded_linux_subprocess(${plugin_test_target})
    endforeach()
    target_include_directories(dandrum-sampler-plugin-host-test PRIVATE
        ${CMAKE_CURRENT_BINARY_DIR}/dandrum-sampler-plugin_artefacts/JuceLibraryCode
        ${CMAKE_CURRENT_BINARY_DIR}/dandrum-plugin_artefacts/JuceLibraryCode)
    target_link_libraries(dandrum-sampler-plugin-host-test PRIVATE juce::pkgconfig_JUCE_BROWSER_LINUX_DEPS)
    juce_link_with_embedded_linux_subprocess(dandrum-sampler-plugin-host-test)
endif()

add_executable(dandrum-drum-loop-pattern-test tests/cpp/DrumLoopPatternTest.cpp)
target_include_directories(dandrum-drum-loop-pattern-test PRIVATE src/juce-wrapper)
target_link_libraries(dandrum-drum-loop-pattern-test PRIVATE)
add_test(NAME drum-loop-pattern-test COMMAND dandrum-drum-loop-pattern-test)
