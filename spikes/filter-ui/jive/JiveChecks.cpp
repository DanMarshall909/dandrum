#include "JiveChecks.h"
#include "ResponseGraph.h"
#include "RotaryControl.h"

namespace filter_spike {
namespace {
template<class T> T* find(juce::Component& parent) {
    if (auto* candidate = dynamic_cast<T*>(&parent)) return candidate;
    for (auto* child : parent.getChildren()) if (auto* candidate = find<T>(*child)) return candidate;
    return nullptr;
}
juce::MouseEvent eventFor(juce::Component& component, juce::Point<float> point, juce::Point<float> start) {
    return { juce::Desktop::getInstance().getMainMouseSource(), point,
             juce::ModifierKeys(juce::ModifierKeys::leftButtonModifier), 1.f, 0.f, 0.f, 0.f, 0.f,
             &component, &component, juce::Time::getCurrentTime(), start, juce::Time::getCurrentTime(), 1, point != start };
}
}
bool checkJiveInteractions(FilterProcessor& processor, juce::AudioProcessorEditor& editor, juce::String& failure) {
    auto* graph = find<jive_ui::ResponseGraph>(editor);
    auto* knob = find<jive_ui::RotaryControl>(editor);
    if (graph == nullptr || knob == nullptr || graph->getWidth() < 200 || graph->getHeight() < 100) {
        failure = "JIVE graph/knob missing or declarative layout collapsed"; return false;
    }
    const auto oldFrequency = processor.normalized(bellFrequency);
    knob->onDragStart();
    knob->setValue(.68, juce::sendNotificationSync);
    knob->onDragEnd();
    if (std::abs(processor.normalized(bellFrequency) - .68f) > .001f || std::abs(oldFrequency - .68f) < .001f) {
        failure = "JIVE rotary gesture did not reach host bell frequency"; return false;
    }
    // Poll the actual painted snapshot while dispatching the same message loop
    // as the host. Sleeping alone can leave JUCE's timer-start message pending.
    const auto x = 48.f + frequencyX(processor.actual(bellFrequency)) * (graph->getWidth() - 72.f);
    const auto y = 42.f + responseY(processor.actual(bellGain)) * (graph->getHeight() - 80.f);
    const juce::Point<float> expected(x, y);
    for (int attempt = 0; attempt < 12 && graph->displayedNodePosition(1).getDistanceFrom(expected) > 1.f; ++attempt) {
#if JUCE_MODAL_LOOPS_PERMITTED
        juce::MessageManager::getInstance()->runDispatchLoopUntil(20);
#else
        juce::Thread::sleep(20);
        juce::Timer::callPendingTimersSynchronously();
#endif
    }
    const auto start = graph->displayedNodePosition(1);
    if (start.getDistanceFrom(expected) > 1.f) {
        failure = "JIVE host polling did not update displayed Bell node: visible=" + juce::String(start.x) + "," + juce::String(start.y)
                + " expected=" + juce::String(expected.x) + "," + juce::String(expected.y)
                + " sequence=" + juce::String(static_cast<juce::int64>(graph->displayedSequence()));
        return false;
    }
    const auto moved = start + juce::Point<float>(-35.f, -18.f);
    auto before = processor.normalized(bellFrequency);
    graph->mouseDown(eventFor(*graph, start, start));
    graph->mouseDrag(eventFor(*graph, moved, start));
    graph->mouseUp(eventFor(*graph, moved, start));
    if (std::abs(processor.normalized(bellFrequency) - before) < .001f) {
        failure = "JIVE graph mouse gesture did not reach host bell frequency: visible=" + juce::String(start.x) + "," + juce::String(start.y)
                + " normalized before=" + juce::String(before) + " after=" + juce::String(processor.normalized(bellFrequency));
        return false;
    }
    return true;
}
}
