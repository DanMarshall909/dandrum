#include "PerformanceView.h"
#include <cmath>
#include <iostream>
#include <set>
#include <stdexcept>

namespace {
using trigger_ui::PerformanceView;
void require(bool condition, const char* message) {
    if (!condition) throw std::runtime_error(message);
}
void near(float actual, float expected, const char* message) {
    if (std::abs(actual - expected) > .0001f)
        throw std::runtime_error(std::string(message) + ": expected " + std::to_string(expected) + ", got " + std::to_string(actual));
}
juce::MouseEvent event(PerformanceView& view, juce::Point<float> point, int flags = juce::ModifierKeys::leftButtonModifier) {
    return {juce::Desktop::getInstance().getMainMouseSource(), point, juce::ModifierKeys(flags),
        1.f, 0.f, 0.f, 0.f, 0.f, &view, &view, juce::Time::getCurrentTime(), point,
        juce::Time::getCurrentTime(), 1, false};
}
void click(PerformanceView& view, juce::Point<float> point, int flags = juce::ModifierKeys::leftButtonModifier, bool rapid = false) {
    // Slint counts clicks using its platform clock, not MouseEvent::getNumberOfClicks().
    if (!rapid) juce::MessageManager::getInstance()->runDispatchLoopUntil(510);
    auto input = event(view, point, flags);
    view.mouseMove(input); view.mouseDown(input); view.mouseUp(input);
}
void key(PerformanceView& view, int code, int modifiers = 0, juce::juce_wchar character = 0) {
    require(view.keyPressed(juce::KeyPress(code, juce::ModifierKeys(modifiers), character)), "Native key was not forwarded");
}
void typed(PerformanceView& view, const juce::String& value) {
    key(view, juce::KeyPress::returnKey);
    juce::MessageManager::getInstance()->runDispatchLoopUntil(10);
    for (auto character : value) key(view, int(character), 0, character);
    key(view, juce::KeyPress::returnKey);
    juce::MessageManager::getInstance()->runDispatchLoopUntil(10);
}
void screenshot(PerformanceView& view, const juce::File& directory, int width) {
    view.setSize(width, 90);
    const auto image = view.createComponentSnapshot(view.getLocalBounds());
    require(image.isValid() && image.getWidth() == width && image.getHeight() == 90, "Native render has wrong size");
    require(image.getPixelAt(0, 0) == juce::Colour(0xff211b16), "Native render lost the React panel palette");
    std::set<juce::uint32> colours;
    for (int y = 0; y < 90; y += 2) for (int x = 0; x < width; x += 2) colours.insert(image.getPixelAt(x, y).getARGB());
    require(colours.size() > 30, "Native render is blank");
    const auto path = directory.getChildFile("native-" + juce::String(width) + ".png");
    auto output = path.createOutputStream();
    require(output != nullptr && output->setPosition(0) && output->truncate().wasOk()
        && juce::PNGImageFormat().writeImageToStream(image, *output), "Cannot write native evidence");
}
}

int main(int argc, char** argv) {
    try {
        require(argc == 2, "Usage: trigger-slint-ui-check <output-directory>");
        juce::ScopedJuceInitialiser_GUI gui;
        juce::Component bootstrap;
        bootstrap.setSize(1, 1); bootstrap.addToDesktop(juce::ComponentPeer::windowIsTemporary);
        bootstrap.setVisible(true);
        juce::MessageManager::getInstance()->runDispatchLoopUntil(20);
        bootstrap.setVisible(false);
        PerformanceView view;
        view.addToDesktop(juce::ComponentPeer::windowIsTemporary); view.setVisible(true);
        juce::MessageManager::getInstance()->runDispatchLoopUntil(30);
        const std::array<float, 8> defaults {.56f, .4f, .32f, .18f, .64f, .5f, 0, 0};
        require(view.macroValues() == defaults, "Native default macro values differ from React");
        const juce::Point<float> tone(205, 49);
        constexpr auto left = juce::ModifierKeys::leftButtonModifier, shift = juce::ModifierKeys::shiftModifier;
        click(view, tone);
        key(view, juce::KeyPress::upKey); near(view.macroValues()[0], .61f, "Arrow step");
        key(view, juce::KeyPress::downKey, shift); near(view.macroValues()[0], .605f, "Precision arrow step");
        key(view, juce::KeyPress::homeKey); near(view.macroValues()[0], 0, "Home endpoint");
        require(view.createComponentSnapshot(view.getLocalBounds()).getPixelAt(189, 47) == juce::Colour(0xff524437),
            "Zero value still renders a filled arc");
        key(view, juce::KeyPress::pageUpKey); near(view.macroValues()[0], .5f, "Page step");
        key(view, juce::KeyPress::endKey); near(view.macroValues()[0], 1, "End endpoint");
        require(view.createComponentSnapshot(view.getLocalBounds()).getPixelAt(189, 47) == juce::Colour(0xfff2e6d3),
            "Full value did not render a filled arc");
        key(view, juce::KeyPress::upKey); near(view.macroValues()[0], 1, "Keyboard clamp");
        click(view, tone, juce::ModifierKeys::middleButtonModifier); near(view.macroValues()[0], .56f, "Middle reset");
        juce::MouseWheelDetails wheel {}; wheel.deltaY = .1f;
        view.mouseWheelMove(event(view, tone, 0), wheel); near(view.macroValues()[0], .58f, "Wheel step");
        view.mouseWheelMove(event(view, tone, shift), wheel); near(view.macroValues()[0], .582f, "Precision wheel step");
        click(view, tone, left | juce::ModifierKeys::altModifier); near(view.macroValues()[0], .56f, "Alt reset");
        juce::MessageManager::getInstance()->runDispatchLoopUntil(510);
        view.mouseDown(event(view, tone));
        view.mouseDrag(event(view, tone + juce::Point<float>(0, -20)));
        near(view.macroValues()[0], .66f, "Native vertical drag");
        view.mouseDrag(event(view, tone + juce::Point<float>(0, -20), left | shift));
        near(view.macroValues()[0], .57f, "Shift during captured drag");
        view.mouseDrag(event(view, tone + juce::Point<float>(0, -30)));
        near(view.macroValues()[0], .71f, "Release Shift during captured drag");
        view.mouseUp(event(view, tone + juce::Point<float>(0, -30)));
        typed(view, "42"); near(view.macroValues()[0], .42f, "Typed percentage");
        typed(view, "oops"); near(view.macroValues()[0], .42f, "Invalid text changed macro");
        typed(view, "125"); near(view.macroValues()[0], 1, "Typed upper clamp");
        typed(view, "-25"); near(view.macroValues()[0], 0, "Typed lower clamp");
        key(view, juce::KeyPress::returnKey); key(view, '3', 0, '3'); key(view, '3', 0, '3');
        key(view, juce::KeyPress::tabKey); near(view.macroValues()[0], .33f, "Leaving the editor did not commit valid text");
        key(view, juce::KeyPress::tabKey, shift); key(view, juce::KeyPress::homeKey);
        key(view, juce::KeyPress::returnKey); key(view, '8', 0, '8'); key(view, juce::KeyPress::escapeKey);
        near(view.macroValues()[0], 0, "Escape did not cancel typed editing");
        key(view, juce::KeyPress::deleteKey); near(view.macroValues()[0], .56f, "Keyboard reset");
        key(view, juce::KeyPress::tabKey); key(view, juce::KeyPress::upKey);
        near(view.macroValues()[1], .45f, "Tab did not reach Body");
        for (int i = 0; i < 5; ++i) key(view, juce::KeyPress::tabKey);
        key(view, juce::KeyPress::upKey); near(view.macroValues()[0], .61f, "Tab did not skip disabled slots");
        key(view, juce::KeyPress::tabKey, shift); key(view, juce::KeyPress::upKey);
        near(view.macroValues()[5], .55f, "Shift-Tab did not reach Width");
        const auto assigned = view.macroValues();
        for (float x : {625.f, 695.f}) {
            click(view, {x, 49}); view.mouseWheelMove(event(view, {x, 49}), wheel);
            require(view.macroValues() == assigned, "Disabled macro accepted pointer/wheel input");
        }
        click(view, tone); click(view, tone, left, true);
        key(view, '3', 0, '3'); key(view, '7', 0, '7'); key(view, juce::KeyPress::returnKey);
        near(view.macroValues()[0], .37f, "Double-click did not open percentage editing");
        click(view, tone);
        juce::MessageManager::getInstance()->runDispatchLoopUntil(510);
        view.mouseDown(event(view, tone));
        view.mouseDrag(event(view, tone + juce::Point<float>(0, -20)));
        view.focusLost(juce::Component::focusChangedDirectly);
        const auto released = view.macroValues();
        view.mouseDrag(event(view, tone + juce::Point<float>(0, -40)));
        require(view.macroValues() == released, "Focus loss retained a captured drag");
        const auto directory = juce::File::getCurrentWorkingDirectory().getChildFile(argv[1]);
        require(directory.createDirectory().wasOk(), "Cannot create evidence directory");
        PerformanceView fresh;
        fresh.addToDesktop(juce::ComponentPeer::windowIsTemporary); fresh.setVisible(true);
        screenshot(fresh, directory, 900); screenshot(fresh, directory, 720);
        const juce::Point<float> narrowTone(115, 49);
        click(fresh, narrowTone); key(fresh, juce::KeyPress::upKey);
        near(fresh.macroValues()[0], .61f, "Resized native knob hit target");
        fresh.setSize(900, 90);
        click(fresh, tone); key(fresh, juce::KeyPress::returnKey);
        const auto focused = fresh.createComponentSnapshot(fresh.getLocalBounds());
        auto output = directory.getChildFile("native-editing.png").createOutputStream();
        require(output && juce::PNGImageFormat().writeImageToStream(focused, *output), "Cannot capture editing state");
        std::cout << "PASS: native defaults, rendering at 900/720, drag/precision, wheel, keys, typing/clamping/cancel, reset, Tab, disabled slots and focus-loss cleanup\n";
        return 0;
    } catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
