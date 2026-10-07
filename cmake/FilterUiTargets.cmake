set(FILTER_SHARED
    spikes/filter-ui/shared/FilterProcessor.cpp
    spikes/filter-ui/shared/FilterAudio.cpp
    spikes/filter-ui/shared/FilterViewModel.cpp
    spikes/filter-ui/shared/FilterAnalysis.cpp)
foreach(target dandrum-filter-slint dandrum-filter-jive)
    target_sources(${target} PRIVATE ${FILTER_SHARED} spikes/filter-ui/shared/PluginEntry.cpp)
    target_include_directories(${target} PRIVATE spikes/filter-ui/shared src/juce-wrapper)
    target_compile_definitions(${target} PRIVATE
        DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}" JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0)
    target_link_libraries(${target} PRIVATE dandrum_engine juce::juce_audio_utils juce::juce_dsp
        PUBLIC juce::juce_recommended_config_flags)
endforeach()
target_sources(dandrum-filter-slint PRIVATE spikes/filter-ui/slint/SlintEditor.cpp)
target_link_libraries(dandrum-filter-slint PRIVATE Slint::Slint)
slint_target_sources(dandrum-filter-slint spikes/filter-ui/slint/Filter.slint)
target_sources(dandrum-filter-jive PRIVATE spikes/filter-ui/jive/JiveEditor.cpp)
target_link_libraries(dandrum-filter-jive PRIVATE jive::jive_layouts jive::jive_style_sheets)
target_compile_definitions(dandrum-filter-jive PRIVATE JIVE_GUI_ITEMS_HAVE_STYLE_SHEETS=1 JIVE_IS_PLUGIN_PROJECT=1)

add_executable(dandrum-filter-spike-processor-test tests/cpp/FilterSpikeProcessorTest.cpp ${FILTER_SHARED})
target_include_directories(dandrum-filter-spike-processor-test PRIVATE spikes/filter-ui/shared src/juce-wrapper)
target_compile_definitions(dandrum-filter-spike-processor-test PRIVATE
    DANDRUM_SOURCE_ROOT="${CMAKE_CURRENT_SOURCE_DIR}" JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0)
target_link_libraries(dandrum-filter-spike-processor-test PRIVATE dandrum_engine juce::juce_audio_utils juce::juce_dsp)
add_test(NAME filter-spike-processor COMMAND dandrum-filter-spike-processor-test)

foreach(backend slint jive)
    string(TOUPPER "${backend}" upper_backend)
    if(backend STREQUAL "slint")
        set(check_source spikes/filter-ui/slint/SlintChecks.cpp)
    else()
        set(check_source spikes/filter-ui/jive/JiveChecks.cpp)
    endif()
    add_executable(dandrum-filter-${backend}-ui-check tests/cpp/FilterSpikeUiCheck.cpp
        ${check_source})
    target_include_directories(dandrum-filter-${backend}-ui-check PRIVATE
        spikes/filter-ui/shared spikes/filter-ui/${backend} src/juce-wrapper)
    target_compile_definitions(dandrum-filter-${backend}-ui-check PRIVATE
        FILTER_SPIKE_${upper_backend}=1 JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_MODAL_LOOPS_PERMITTED=1)
    target_link_libraries(dandrum-filter-${backend}-ui-check PRIVATE dandrum-filter-${backend}
        juce::juce_audio_utils juce::juce_dsp)
    add_test(NAME filter-${backend}-ui COMMAND dandrum-filter-${backend}-ui-check
        "${CMAKE_CURRENT_BINARY_DIR}/filter-ui-evidence/${backend}")
endforeach()
target_link_libraries(dandrum-filter-slint-ui-check PRIVATE Slint::Slint)
target_link_libraries(dandrum-filter-jive-ui-check PRIVATE jive::jive_layouts jive::jive_style_sheets)
target_compile_definitions(dandrum-filter-jive-ui-check PRIVATE JIVE_GUI_ITEMS_HAVE_STYLE_SHEETS=1
    JIVE_IS_PLUGIN_PROJECT=1)

set(FILTER_VST3_CHECK_TARGET)
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(X11 REQUIRED)
    add_executable(dandrum-filter-spike-vst3-test tests/cpp/FilterSpikeVst3Test.cpp)
    target_compile_definitions(dandrum-filter-spike-vst3-test PRIVATE JUCE_PLUGINHOST_VST3=1
        JUCE_MODAL_LOOPS_PERMITTED=1 JUCE_STANDALONE_APPLICATION=1 JUCE_USE_CURL=0 JUCE_WEB_BROWSER=0)
    target_link_libraries(dandrum-filter-spike-vst3-test PRIVATE juce::juce_audio_utils X11::X11)
    foreach(backend slint jive)
        get_target_property(bundle dandrum-filter-${backend}_VST3 JUCE_PLUGIN_ARTEFACT_FILE)
        add_test(NAME filter-${backend}-vst3 COMMAND dandrum-filter-spike-vst3-test
            "${bundle}" "${CMAKE_CURRENT_BINARY_DIR}/filter-ui-evidence/${backend}-vst3")
    endforeach()
    set(FILTER_VST3_CHECK_TARGET dandrum-filter-spike-vst3-test)
endif()
add_custom_target(dandrum-filter-ui-spikes-all DEPENDS
    dandrum-filter-slint_Standalone dandrum-filter-slint_VST3 dandrum-filter-jive_Standalone dandrum-filter-jive_VST3
    dandrum-filter-slint-ui-check dandrum-filter-jive-ui-check ${FILTER_VST3_CHECK_TARGET}
    dandrum-filter-spike-processor-test dandrum-filter-spike-engine-test)
