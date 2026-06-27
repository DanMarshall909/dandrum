cmake_minimum_required(VERSION 3.22)

project(Dandrum VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

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
        juce::juce_audio_basics
        juce::juce_audio_devices
        juce::juce_core
    PUBLIC
        juce::juce_recommended_config_flags
        juce::juce_recommended_lto_flags
        juce::juce_recommended_warning_flags)
