cmake_minimum_required(VERSION 3.22)

if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
    project(DandrumSlintLibrary VERSION 0.1.0 LANGUAGES CXX)
    enable_testing()
endif()

# Match the design library's compiler/runtime verification version.
if(NOT TARGET Slint::Slint)
    find_package(Slint 1.18.1 EXACT CONFIG REQUIRED)
endif()
set(DANDRUM_SLINT_ROOT "${CMAKE_CURRENT_SOURCE_DIR}")
get_filename_component(DANDRUM_REPO_ROOT "${DANDRUM_SLINT_ROOT}/../.." ABSOLUTE)

add_library(dandrum-slint-library INTERFACE)
add_library(Dandrum::SlintLibrary ALIAS dandrum-slint-library)
target_compile_features(dandrum-slint-library INTERFACE cxx_std_20)
target_include_directories(dandrum-slint-library INTERFACE "${DANDRUM_SLINT_ROOT}")
target_link_libraries(dandrum-slint-library INTERFACE Slint::Slint)

option(DANDRUM_SLINT_BUILD_FILE_DROP "Build local file URI parsing and optional X11 transport" ON)
if(DANDRUM_SLINT_BUILD_FILE_DROP)
    add_subdirectory(platform)
endif()

option(DANDRUM_SLINT_BUILD_CATALOG "Build the interactive design catalog" ON)
if(DANDRUM_SLINT_BUILD_CATALOG)
    if(NOT TARGET dandrum-slint-catalog)
        add_executable(dandrum-slint-catalog)
    endif()
    target_sources(dandrum-slint-catalog PRIVATE catalog/Main.cpp)
    target_link_libraries(dandrum-slint-catalog PRIVATE Dandrum::SlintLibrary)
    set_target_properties(dandrum-slint-catalog PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/slint-catalog/$<CONFIG>"
        SLINT_EMBED_RESOURCES embed-files)
    slint_target_sources(dandrum-slint-catalog catalog/Catalog.slint NAMESPACE dandrum_ui)
    add_custom_target(dandrum-slint-catalog-font-notices)
    add_dependencies(dandrum-slint-catalog dandrum-slint-catalog-font-notices)
    foreach(family Barlow BarlowSemiCondensed JetBrainsMono)
        add_custom_command(TARGET dandrum-slint-catalog-font-notices POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E make_directory "$<TARGET_FILE_DIR:dandrum-slint-catalog>/font-licenses/${family}"
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${DANDRUM_REPO_ROOT}/ui/design-system/fonts/${family}/OFL.txt"
                "$<TARGET_FILE_DIR:dandrum-slint-catalog>/font-licenses/${family}/OFL.txt")
    endforeach()
    # The root launcher declares the executable in its directory. Make the generated
    # sources an explicit dependency across that CMake directory boundary as well.
    add_custom_target(dandrum-slint-catalog-generated
        DEPENDS "${CMAKE_CURRENT_BINARY_DIR}/Catalog.h"
                "${CMAKE_CURRENT_BINARY_DIR}/slint_generated_Catalog_1.cpp")
    add_dependencies(dandrum-slint-catalog dandrum-slint-catalog-generated)
endif()

option(DANDRUM_SLINT_BUILD_TESTS "Build the native value codec and generated binding tests" ON)
option(DANDRUM_SLINT_TEST_REPOSITORY "Include Node/Python repository integration gates (requires repository dev dependencies)" OFF)
option(DANDRUM_SLINT_TEST_RUNTIME "Register live viewer interaction and screenshot tests (requires slint-viewer)" OFF)
option(DANDRUM_SLINT_TEST_HEADLESS "Run registered live viewer tests on a private Linux Xvfb display" OFF)
if(DANDRUM_SLINT_BUILD_TESTS)
    if(DANDRUM_SLINT_TEST_REPOSITORY)
        find_program(DANDRUM_SLINT_NODE node REQUIRED)
        add_test(NAME slint-token-drift COMMAND "${DANDRUM_SLINT_NODE}"
            "${DANDRUM_REPO_ROOT}/scripts/generate-ui-tokens.mjs" --check)
        add_test(NAME sampler-command-drift COMMAND "${DANDRUM_SLINT_NODE}"
            "${DANDRUM_REPO_ROOT}/scripts/generate-sampler-commands.mjs" --check)
        add_test(NAME sampler-command-contracts COMMAND "${DANDRUM_SLINT_NODE}" --test
            "${DANDRUM_REPO_ROOT}/tests/js/SamplerCommandsTest.mjs")
        add_test(NAME slint-token-contracts COMMAND "${DANDRUM_SLINT_NODE}" --test
            "${DANDRUM_REPO_ROOT}/tests/js/DesignTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintDesignTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintEditorChromeTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintEditorControlTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintEditorDisplayTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintEditorRootTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintEditorLayoutTokensTest.mjs")
        add_test(NAME slint-library-inventory COMMAND "${DANDRUM_SLINT_NODE}" --test
            "${DANDRUM_REPO_ROOT}/tests/js/SlintLibraryInventoryTest.mjs")
        find_package(Python3 COMPONENTS Interpreter REQUIRED)
        add_test(NAME slint-editor-package COMMAND "${DANDRUM_SLINT_NODE}" --test
            "${DANDRUM_REPO_ROOT}/tests/js/SlintEditorPackageTest.mjs")
        add_test(NAME slint-public-api-consumer COMMAND "${Python3_EXECUTABLE}"
            "${DANDRUM_REPO_ROOT}/tests/cmake/slint-editor-consumer/run_consumer.py"
            --cmake "${CMAKE_COMMAND}" --sdk "${Slint_DIR}"
            --evidence "${CMAKE_BINARY_DIR}/slint-public-api-consumer")
        set_tests_properties(slint-public-api-consumer PROPERTIES
            ENVIRONMENT "PYTHONDONTWRITEBYTECODE=1" TIMEOUT 420 LABELS "slint;editor;package")
        add_test(NAME demo-launcher COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_demo_launcher.py)
        add_test(NAME demo-launcher-inventory COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_demo_inventory.py)
        add_test(NAME demo-launcher-boundaries COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_demo_launcher_boundaries.py)
        add_test(NAME slint-headless-launch COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_slint_headless.py)
        set_tests_properties(slint-headless-launch PROPERTIES ENVIRONMENT "PYTHONDONTWRITEBYTECODE=1")
    endif()
    if(DANDRUM_SLINT_TEST_RUNTIME)
        find_package(Python3 COMPONENTS Interpreter REQUIRED)
        find_program(DANDRUM_SLINT_VIEWER slint-viewer REQUIRED)
        set(DANDRUM_SLINT_RUNTIME_ARGUMENTS)
        if(DANDRUM_SLINT_TEST_HEADLESS)
            list(APPEND DANDRUM_SLINT_RUNTIME_ARGUMENTS --headless)
        endif()
        add_test(NAME slint-library-runtime COMMAND "${Python3_EXECUTABLE}"
            "${DANDRUM_REPO_ROOT}/scripts/check-slint-library.py"
            --viewer "${DANDRUM_SLINT_VIEWER}"
            --evidence "${CMAKE_BINARY_DIR}/slint-library-evidence"
            ${DANDRUM_SLINT_RUNTIME_ARGUMENTS})
        set_tests_properties(slint-library-runtime PROPERTIES
            ENVIRONMENT "PYTHONDONTWRITEBYTECODE=1" TIMEOUT 600 LABELS "slint;runtime")
        if(TARGET dandrum-slint-x11-file-drop)
            find_package(X11 REQUIRED)
            add_executable(dandrum-slint-file-drop-transport-test
                "${DANDRUM_REPO_ROOT}/tests/cpp/SlintFileDropTransportTest.cpp")
            target_link_libraries(dandrum-slint-file-drop-transport-test PRIVATE
                Dandrum::SlintLibrary Dandrum::SlintX11FileDrop X11::X11)
            slint_target_sources(dandrum-slint-file-drop-transport-test
                "${DANDRUM_REPO_ROOT}/tests/slint/AdvancedSamplerNativeFileDrop.slint" NAMESPACE dandrum_ui)
            add_test(NAME slint-file-drop-transport COMMAND "${Python3_EXECUTABLE}"
                "${DANDRUM_REPO_ROOT}/scripts/run-native-slint-test.py"
                --executable "$<TARGET_FILE:dandrum-slint-file-drop-transport-test>"
                --evidence "${CMAKE_BINARY_DIR}/slint-file-drop-transport"
                ${DANDRUM_SLINT_RUNTIME_ARGUMENTS})
            set_tests_properties(slint-file-drop-transport PROPERTIES
                ENVIRONMENT "PYTHONDONTWRITEBYTECODE=1" TIMEOUT 120 LABELS "slint;runtime;file-drop")
        endif()
    endif()
    add_executable(dandrum-slint-value-codec-test "${DANDRUM_REPO_ROOT}/tests/cpp/SlintValueCodecTest.cpp")
    target_compile_features(dandrum-slint-value-codec-test PRIVATE cxx_std_20)
    add_test(NAME slint-value-codec COMMAND dandrum-slint-value-codec-test)
    add_executable(dandrum-slint-model-projection-test
        "${DANDRUM_REPO_ROOT}/tests/cpp/SlintModelProjectionTest.cpp")
    target_include_directories(dandrum-slint-model-projection-test PRIVATE "${DANDRUM_REPO_ROOT}")
    target_link_libraries(dandrum-slint-model-projection-test PRIVATE Dandrum::SlintLibrary)
    add_test(NAME slint-model-projection COMMAND dandrum-slint-model-projection-test)
    if(TARGET dandrum-slint-file-uris)
        add_executable(dandrum-slint-file-uris-test "${DANDRUM_REPO_ROOT}/tests/cpp/SlintFileUrisTest.cpp")
        target_link_libraries(dandrum-slint-file-uris-test PRIVATE Dandrum::SlintFileUris)
        add_test(NAME slint-file-uris COMMAND dandrum-slint-file-uris-test)
    endif()

    get_target_property(DANDRUM_SLINT_FEATURES Slint::Slint SLINT_ENABLED_FEATURES)
    if("TESTING" IN_LIST DANDRUM_SLINT_FEATURES)
        # The headless test needs element IDs emitted by the compiler. Keep this
        # instrumentation local to the test; the catalog remains a normal build.
        set(DANDRUM_SLINT_BINDING_DIR "${CMAKE_CURRENT_BINARY_DIR}/binding-test")
        file(MAKE_DIRECTORY "${DANDRUM_SLINT_BINDING_DIR}")
        add_custom_command(
            OUTPUT "${DANDRUM_SLINT_BINDING_DIR}/Controls.h"
            COMMAND ${CMAKE_COMMAND} -E env SLINT_EMIT_DEBUG_INFO=1
                $<TARGET_FILE:Slint::slint-compiler>
                "${DANDRUM_REPO_ROOT}/tests/slint/Controls.slint"
                -f cpp --cpp-namespace=dandrum_ui --embed-resources=embed-files
                --depfile "${DANDRUM_SLINT_BINDING_DIR}/Controls.d"
                -o "${DANDRUM_SLINT_BINDING_DIR}/Controls.h"
            DEPENDS Slint::slint-compiler "${DANDRUM_REPO_ROOT}/tests/slint/Controls.slint"
            DEPFILE "${DANDRUM_SLINT_BINDING_DIR}/Controls.d"
            VERBATIM)
        add_executable(dandrum-slint-codec-binding-test
            "${DANDRUM_REPO_ROOT}/tests/cpp/SlintCodecBindingTest.cpp"
            "${DANDRUM_SLINT_BINDING_DIR}/Controls.h")
        target_include_directories(dandrum-slint-codec-binding-test PRIVATE
            "${DANDRUM_REPO_ROOT}" "${DANDRUM_SLINT_BINDING_DIR}")
        target_compile_definitions(dandrum-slint-codec-binding-test PRIVATE SLINT_FEATURE_EXPERIMENTAL=1)
        target_link_libraries(dandrum-slint-codec-binding-test PRIVATE Dandrum::SlintLibrary)
        add_test(NAME slint-codec-binding COMMAND dandrum-slint-codec-binding-test)
        add_executable(dandrum-slint-control-interactions-test
            "${DANDRUM_REPO_ROOT}/tests/cpp/SlintControlInteractionsTest.cpp")
        target_include_directories(dandrum-slint-control-interactions-test PRIVATE "${DANDRUM_REPO_ROOT}")
        target_compile_definitions(dandrum-slint-control-interactions-test PRIVATE SLINT_FEATURE_EXPERIMENTAL=1)
        target_link_libraries(dandrum-slint-control-interactions-test PRIVATE Dandrum::SlintLibrary)
        set(saved_slint_environment "${SLINT_COMPILER_ENV_VARS}")
        list(APPEND SLINT_COMPILER_ENV_VARS SLINT_EMIT_DEBUG_INFO=1)
        slint_target_sources(dandrum-slint-control-interactions-test
            "${DANDRUM_REPO_ROOT}/tests/slint/ControlInteractions.slint" NAMESPACE dandrum_ui)
        set(SLINT_COMPILER_ENV_VARS "${saved_slint_environment}")
        add_test(NAME slint-control-interactions COMMAND dandrum-slint-control-interactions-test)
        # Independent consumers prove editor primitives do not need sampler state.
        set(editor_render_fixtures Displays Plots ZoneMap Collections Chrome ModulationLayers)
        foreach(editor_fixture IN ITEMS ThemeControls Parameters Displays Plots Dialogs Browser
                                        ZoneMap Collections Chrome ModulationLayers)
            string(TOLOWER "${editor_fixture}" editor_kind)
            set(editor_target "dandrum-slint-editor-${editor_kind}-test")
            add_executable(${editor_target}
                "${DANDRUM_REPO_ROOT}/tests/cpp/SlintEditor${editor_fixture}Test.cpp")
            target_include_directories(${editor_target} PRIVATE "${DANDRUM_REPO_ROOT}")
            target_compile_definitions(${editor_target} PRIVATE SLINT_FEATURE_EXPERIMENTAL=1)
            target_link_libraries(${editor_target} PRIVATE Dandrum::SlintLibrary)
            set(saved_editor_environment "${SLINT_COMPILER_ENV_VARS}")
            list(APPEND SLINT_COMPILER_ENV_VARS SLINT_EMIT_DEBUG_INFO=1)
            slint_target_sources(${editor_target}
                "${DANDRUM_REPO_ROOT}/tests/slint/SlintEditor${editor_fixture}.slint" NAMESPACE dandrum_ui)
            set(SLINT_COMPILER_ENV_VARS "${saved_editor_environment}")
            add_test(NAME slint-editor-${editor_kind} COMMAND ${editor_target})
            set_tests_properties(slint-editor-${editor_kind} PROPERTIES LABELS "slint;editor;interaction")
            if(DANDRUM_SLINT_TEST_RUNTIME AND editor_fixture IN_LIST editor_render_fixtures)
                add_test(NAME slint-editor-${editor_kind}-render COMMAND "${Python3_EXECUTABLE}"
                    "${DANDRUM_REPO_ROOT}/scripts/run-native-slint-test.py"
                    --executable "$<TARGET_FILE:${editor_target}>"
                    --evidence "${CMAKE_BINARY_DIR}/slint-editor-${editor_kind}-render"
                    ${DANDRUM_SLINT_RUNTIME_ARGUMENTS})
                set_tests_properties(slint-editor-${editor_kind}-render PROPERTIES
                    ENVIRONMENT "PYTHONDONTWRITEBYTECODE=1" TIMEOUT 120 LABELS "slint;editor;render")
            endif()
        endforeach()
    else()
        message(STATUS "Slint SDK has no TESTING support: generated binding test unavailable")
    endif()
endif()
