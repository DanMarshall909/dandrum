#pragma once
#include <juce_gui_basics/juce_gui_basics.h>
#include <slint.h>
#include <array>

namespace trigger_ui {
// Slint's platform API accepts the documented Unicode representation of each key.
inline slint::SharedString keyText(const juce::KeyPress& key) {
    const std::array<std::pair<int, const char*>, 13> special {{
        {juce::KeyPress::upKey, "\uF700"}, {juce::KeyPress::downKey, "\uF701"},
        {juce::KeyPress::leftKey, "\uF702"}, {juce::KeyPress::rightKey, "\uF703"},
        {juce::KeyPress::homeKey, "\uF729"}, {juce::KeyPress::endKey, "\uF72B"},
        {juce::KeyPress::pageUpKey, "\uF72C"}, {juce::KeyPress::pageDownKey, "\uF72D"},
        {juce::KeyPress::returnKey, "\n"}, {juce::KeyPress::escapeKey, "\x1b"},
        {juce::KeyPress::deleteKey, "\x7f"}, {juce::KeyPress::backspaceKey, "\x08"},
        {juce::KeyPress::tabKey, key.getModifiers().isShiftDown() ? "\x19" : "\t"}}};
    for (const auto& [code, text] : special) if (key.getKeyCode() == code) return text;
    return juce::String::charToString(key.getTextCharacter()).toRawUTF8();
}

class Modifiers {
public:
    void sync(slint::Window& window, juce::ModifierKeys modifiers) {
        const std::array<bool, 4> next {modifiers.isShiftDown(), modifiers.isCtrlDown(),
            modifiers.isAltDown(), modifiers.isCommandDown()};
        constexpr std::array<const char*, 4> keys {"\x10", "\x11", "\x12", "\x17"};
        for (size_t i = 0; i < next.size(); ++i) if (next[i] != current[i]) {
            if (next[i]) window.dispatch_key_press_event(keys[i]);
            else window.dispatch_key_release_event(keys[i]);
        }
        current = next;
    }
private:
    std::array<bool, 4> current {};
};
}
