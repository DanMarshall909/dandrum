#include "PerformanceView.h"
#include "Input.h"
#include "SlintPlatform.h"
#include "App.h"
#include <optional>

namespace trigger_ui {
struct PerformanceView::Impl {
    explicit Impl(PerformanceView& owner) : ui(create(owner)) { ui->show(); }
    ~Impl() { ui->hide(); }
    slint::ComponentHandle<PerformanceUi> create(PerformanceView& owner) {
        auto& platform = filter_spike::JuceSlintPlatform::get();
        filter_spike::JuceSlintPlatform::Association association(platform, owner, adapter);
        return PerformanceUi::create();
    }
    static slint::LogicalPosition position(const juce::MouseEvent& event) {
        return slint::LogicalPosition({event.position.x, event.position.y});
    }
    slint::Window& window() { return adapter->window(); }
    filter_spike::SlintWindow* adapter = nullptr; // Borrowed; owned by the Slint window.
    slint::ComponentHandle<PerformanceUi> ui;
    Modifiers modifiers;
    std::optional<slint::PointerEventButton> pressed;
    slint::LogicalPosition lastPosition {{0, 0}};
};

PerformanceView::PerformanceView() {
    setSize(900, 90);
    setWantsKeyboardFocus(true);
    impl = std::make_unique<Impl>(*this);
    resized();
    startTimerHz(60);
}
PerformanceView::~PerformanceView() { stopTimer(); }
void PerformanceView::paint(juce::Graphics& graphics) {
    slint::platform::update_timers_and_animations();
    impl->adapter->paint(graphics);
}
void PerformanceView::resized() {
    if (!impl) return;
    impl->ui->set_viewport_width(float(getWidth()));
    impl->ui->set_viewport_height(float(getHeight()));
    impl->window().dispatch_resize_event(slint::LogicalSize({float(getWidth()), float(getHeight())}));
}
void PerformanceView::mouseDown(const juce::MouseEvent& event) {
    grabKeyboardFocus();
    impl->modifiers.sync(impl->window(), event.mods);
    impl->pressed = event.mods.isMiddleButtonDown() ? slint::PointerEventButton::Middle
        : event.mods.isRightButtonDown() ? slint::PointerEventButton::Right : slint::PointerEventButton::Left;
    impl->lastPosition = Impl::position(event);
    impl->window().dispatch_pointer_press_event(impl->lastPosition, *impl->pressed);
}
void PerformanceView::mouseUp(const juce::MouseEvent& event) {
    impl->modifiers.sync(impl->window(), event.mods);
    if (impl->pressed) impl->window().dispatch_pointer_release_event(Impl::position(event), *impl->pressed);
    impl->pressed.reset();
}
void PerformanceView::mouseMove(const juce::MouseEvent& event) {
    impl->modifiers.sync(impl->window(), event.mods);
    impl->lastPosition = Impl::position(event);
    impl->window().dispatch_pointer_move_event(impl->lastPosition);
}
void PerformanceView::mouseDrag(const juce::MouseEvent& event) { mouseMove(event); }
void PerformanceView::mouseExit(const juce::MouseEvent&) { impl->window().dispatch_pointer_exit_event(); }
void PerformanceView::mouseWheelMove(const juce::MouseEvent& event, const juce::MouseWheelDetails& wheel) {
    impl->modifiers.sync(impl->window(), event.mods);
    impl->window().dispatch_pointer_scroll_event(Impl::position(event), wheel.deltaX * 120, wheel.deltaY * 120);
}
bool PerformanceView::keyPressed(const juce::KeyPress& key) {
    impl->modifiers.sync(impl->window(), key.getModifiers());
    auto text = keyText(key);
    if (text.empty()) return false;
    impl->window().dispatch_key_press_event(text);
    impl->window().dispatch_key_release_event(text);
    return true;
}
void PerformanceView::modifierKeysChanged(const juce::ModifierKeys& modifiers) {
    impl->modifiers.sync(impl->window(), modifiers);
}
void PerformanceView::focusGained(FocusChangeType) { impl->window().dispatch_window_active_changed_event(true); }
void PerformanceView::focusLost(FocusChangeType) {
    if (impl->pressed) impl->window().dispatch_pointer_release_event(impl->lastPosition, *impl->pressed);
    impl->pressed.reset();
    impl->modifiers.sync(impl->window(), {});
    impl->window().dispatch_window_active_changed_event(false);
}
void PerformanceView::timerCallback() { slint::platform::update_timers_and_animations(); }
std::array<float, 8> PerformanceView::macroValues() const {
    std::array<float, 8> values {};
    for (size_t i = 0; i < values.size(); ++i) values[i] = impl->ui->get_values()->row_data(i).value();
    return values;
}
}
