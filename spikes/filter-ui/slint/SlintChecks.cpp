#include "SlintChecks.h"
#include "FilterViewModel.h"

namespace filter_spike {
namespace {
juce::MouseEvent eventFor(juce::Component& component, juce::Point<float> point, juce::Point<float> start) {
    return {juce::Desktop::getInstance().getMainMouseSource(), point,
        juce::ModifierKeys(juce::ModifierKeys::leftButtonModifier), 1.f, 0.f, 0.f, 0.f, 0.f,
        &component, &component, juce::Time::getCurrentTime(), start, juce::Time::getCurrentTime(), 1, point != start};
}
void drag(juce::AudioProcessorEditor& editor, juce::Point<float> start, juce::Point<float> finish) {
    editor.mouseMove(eventFor(editor, start, start));
    editor.mouseDown(eventFor(editor, start, start));
    editor.mouseDrag(eventFor(editor, finish, start));
    // Pump a frame while captured to verify that model updates preserve the TouchArea.
    juce::Thread::sleep(40);
    juce::Timer::callPendingTimersSynchronously();
    editor.mouseDrag(eventFor(editor, finish + juce::Point<float>(2, -2), start));
    editor.mouseUp(eventFor(editor, finish + juce::Point<float>(2, -2), start));
}
void click(juce::AudioProcessorEditor& editor, juce::Point<float> point) {
    editor.mouseMove(eventFor(editor, point, point));
    editor.mouseDown(eventFor(editor, point, point));
    editor.mouseUp(eventFor(editor, point, point));
}
}
bool checkSlintInteractions(FilterProcessor& processor, juce::AudioProcessorEditor& editor, juce::String& error) {
    auto geometry = readSlintCheckGeometry(editor);
    if (geometry.graph.getWidth() < 200 || geometry.frequencyKnob.x <= 0) {
        error = "Slint declarative graph/knob layout collapsed"; return false;
    }
    if (!editor.getLocalBounds().toFloat().contains(geometry.aboutButton)
        || !editor.getLocalBounds().toFloat().contains(geometry.frequencyKnob)
        || !editor.getLocalBounds().toFloat().contains(geometry.graph)) {
        error = "Slint Window layout extends beyond JUCE editor viewport"; return false;
    }
    const auto graph = geometry.graph;
    const juce::Point<float> bell(graph.getX() + frequencyX(processor.actual(bellFrequency)) * graph.getWidth(),
        graph.getY() + responseY(processor.actual(bellGain)) * graph.getHeight());
    const auto initialFrequency = processor.normalized(bellFrequency);
    const auto initialGain = processor.normalized(bellGain);
    drag(editor, bell, bell + juce::Point<float>(48, -24));
    if (processor.normalized(bellFrequency) <= initialFrequency + .001f
        || processor.normalized(bellGain) <= initialGain + .001f) {
        error = "Slint Bell Path node mouse gesture did not update frequency and gain"; return false;
    }
    geometry = readSlintCheckGeometry(editor);
    const auto beforeKnob = processor.normalized(bellFrequency);
    drag(editor, geometry.frequencyKnob, geometry.frequencyKnob + juce::Point<float>(0, -30));
    if (processor.normalized(bellFrequency) <= beforeKnob + .05f) {
        error = "Slint frequency knob mouse gesture did not reach host state"; return false;
    }
    geometry = readSlintCheckGeometry(editor);
    click(editor, geometry.aboutButton);
    if (!readSlintCheckGeometry(editor).aboutOpen) {
        error = "Slint About attribution screen did not open from click at "
            + juce::String(geometry.aboutButton.x) + "," + juce::String(geometry.aboutButton.y)
            + " editor " + juce::String(editor.getWidth()) + "x" + juce::String(editor.getHeight());
        return false;
    }
    click(editor, readSlintCheckGeometry(editor).aboutClose);
    if (readSlintCheckGeometry(editor).aboutOpen) {
        error = "Slint About attribution screen did not close from click"; return false;
    }
    return true;
}
}
