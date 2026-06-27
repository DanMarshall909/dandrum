cmake_minimum_required(VERSION 3.22)

project(Dandrum VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(RUST_ENGINE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/rust-engine)
set(RUST_ENGINE_LIB ${CMAKE_CURRENT_BINARY_DIR}/rust-target/release/libdandrum_engine.a)
set(CARGO_BIN $ENV{HOME}/.cargo/bin/cargo)

add_custom_command(
    OUTPUT ${RUST_ENGINE_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rust-target ${CARGO_BIN} build --release
    WORKING_DIRECTORY ${RUST_ENGINE_DIR}
    DEPENDS
        ${RUST_ENGINE_DIR}/Cargo.toml
        ${RUST_ENGINE_DIR}/src/lib.rs
    COMMENT "Building Rust audio engine")

add_custom_target(dandrum-engine-rust DEPENDS ${RUST_ENGINE_LIB})

add_library(dandrum_engine STATIC IMPORTED GLOBAL)
set_target_properties(dandrum_engine PROPERTIES IMPORTED_LOCATION ${RUST_ENGINE_LIB})
add_dependencies(dandrum_engine dandrum-engine-rust)

add_subdirectory(third_party/JUCE)

juce_add_console_app(dandrum-beep
    PRODUCT_NAME "Dandrum Beep")

target_sources(dandrum-beep
    PRIVATE
        src/juce-wrapper/Main.cpp)

target_compile_definitions(dandrum-beep
    PRIVATE
        JUCE_USE_CURL=0
        JUCE_WEB_BROWSER=0)

target_link_libraries(dandrum-beep
    PRIVATE
        dandrum_engine
        juce::juce_audio_basics
        juce::juce_audio_devices
        juce::juce_core
    PUBLIC
        juce::juce_recommended_config_flags
        juce::juce_recommended_lto_flags
        juce::juce_recommended_warning_flags)
