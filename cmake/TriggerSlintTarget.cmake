add_library(dandrum-trigger-slint-view STATIC spikes/trigger-slint/src/PerformanceView.cpp)
target_include_directories(dandrum-trigger-slint-view PUBLIC spikes/trigger-slint/src
    PRIVATE spikes/filter-ui/slint)
target_compile_definitions(dandrum-trigger-slint-view PUBLIC JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0)
target_link_libraries(dandrum-trigger-slint-view PUBLIC Slint::Slint juce::juce_gui_basics
    juce::juce_recommended_config_flags)
slint_target_sources(dandrum-trigger-slint-view spikes/trigger-slint/ui/App.slint NAMESPACE trigger_ui)
target_sources(dandrum-trigger-slint PRIVATE spikes/trigger-slint/src/Main.cpp)
target_link_libraries(dandrum-trigger-slint PRIVATE dandrum-trigger-slint-view)

add_executable(dandrum-trigger-slint-ui-check tests/cpp/TriggerSlintUiCheck.cpp)
target_compile_definitions(dandrum-trigger-slint-ui-check PRIVATE JUCE_MODAL_LOOPS_PERMITTED=1)
target_link_libraries(dandrum-trigger-slint-ui-check PRIVATE dandrum-trigger-slint-view)
add_test(NAME trigger-slint-ui COMMAND dandrum-trigger-slint-ui-check
    "${CMAKE_CURRENT_BINARY_DIR}/trigger-slint-evidence")
set_tests_properties(trigger-slint-ui PROPERTIES TIMEOUT 30 RUN_SERIAL TRUE LABELS "runtime")
