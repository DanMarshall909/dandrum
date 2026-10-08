cmake_minimum_required(VERSION 3.22)

if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
    project(DandrumSlintLibrary VERSION 0.1.0 LANGUAGES CXX)
    enable_testing()
endif()

# Match the design library's compiler/runtime verification version.
find_package(Slint 1.18.1 EXACT CONFIG REQUIRED)
set(DANDRUM_SLINT_ROOT "${CMAKE_CURRENT_SOURCE_DIR}")
get_filename_component(DANDRUM_REPO_ROOT "${DANDRUM_SLINT_ROOT}/../.." ABSOLUTE)

add_library(dandrum-slint-library INTERFACE)
add_library(Dandrum::SlintLibrary ALIAS dandrum-slint-library)
target_compile_features(dandrum-slint-library INTERFACE cxx_std_20)
target_include_directories(dandrum-slint-library INTERFACE "${DANDRUM_SLINT_ROOT}")
target_link_libraries(dandrum-slint-library INTERFACE Slint::Slint)

if(NOT TARGET dandrum-slint-catalog)
    add_executable(dandrum-slint-catalog)
endif()
target_sources(dandrum-slint-catalog PRIVATE catalog/Main.cpp)
target_link_libraries(dandrum-slint-catalog PRIVATE Dandrum::SlintLibrary)
set_target_properties(dandrum-slint-catalog PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/slint-catalog/$<CONFIG>"
    SLINT_EMBED_RESOURCES embed-files)
slint_target_sources(dandrum-slint-catalog catalog/Catalog.slint NAMESPACE dandrum_ui)
# The root launcher declares the executable in its directory. Make the generated
# sources an explicit dependency across that CMake directory boundary as well.
add_custom_target(dandrum-slint-catalog-generated
    DEPENDS "${CMAKE_CURRENT_BINARY_DIR}/Catalog.h"
            "${CMAKE_CURRENT_BINARY_DIR}/slint_generated_Catalog_1.cpp")
add_dependencies(dandrum-slint-catalog dandrum-slint-catalog-generated)

option(DANDRUM_SLINT_BUILD_TESTS "Build the native value codec and generated binding tests" ON)
option(DANDRUM_SLINT_TEST_REPOSITORY "Include Node/Python repository integration gates (requires repository dev dependencies)" OFF)
if(DANDRUM_SLINT_BUILD_TESTS)
    if(DANDRUM_SLINT_TEST_REPOSITORY)
        find_program(DANDRUM_SLINT_NODE node REQUIRED)
        add_test(NAME slint-token-drift COMMAND "${DANDRUM_SLINT_NODE}"
            "${DANDRUM_REPO_ROOT}/scripts/generate-ui-tokens.mjs" --check)
        add_test(NAME slint-token-contracts COMMAND "${DANDRUM_SLINT_NODE}" --test
            "${DANDRUM_REPO_ROOT}/tests/js/DesignTokensTest.mjs"
            "${DANDRUM_REPO_ROOT}/tests/js/SlintDesignTokensTest.mjs")
        add_test(NAME slint-library-inventory COMMAND "${DANDRUM_SLINT_NODE}" --test
            "${DANDRUM_REPO_ROOT}/tests/js/SlintLibraryInventoryTest.mjs")
        find_package(Python3 COMPONENTS Interpreter REQUIRED)
        add_test(NAME demo-launcher COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_demo_launcher.py)
        add_test(NAME demo-launcher-inventory COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_demo_inventory.py)
        add_test(NAME demo-launcher-boundaries COMMAND "${Python3_EXECUTABLE}" -m unittest discover
            -s "${DANDRUM_REPO_ROOT}/tests/python" -p test_demo_launcher_boundaries.py)
    endif()
    add_executable(dandrum-slint-value-codec-test "${DANDRUM_REPO_ROOT}/tests/cpp/SlintValueCodecTest.cpp")
    target_compile_features(dandrum-slint-value-codec-test PRIVATE cxx_std_20)
    add_test(NAME slint-value-codec COMMAND dandrum-slint-value-codec-test)

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
    else()
        message(STATUS "Slint SDK has no TESTING support: generated binding test unavailable")
    endif()
endif()
