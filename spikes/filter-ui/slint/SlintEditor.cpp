#include "FilterViewModel.h"
#include "SlintPlatform.h"
#include "Filter.h"
#include "SlintChecks.h"
#include <sstream>

namespace filter_spike {
namespace {
slint::SharedString pathCommand(const std::array<float, plotBins>& values, float low, float high) {
    std::ostringstream path;
    path.precision(5);
    for (int i = 0; i < plotBins; ++i) {
        const auto y = 1000.0f * juce::jlimit(0.0f, 1.0f, (high - values[size_t(i)]) / (high - low));
        path << (i == 0 ? "M " : " L ") << float(i) * 1000.0f / float(plotBins - 1) << ' ' << y;
    }
    return slint::SharedString(path.str());
}
template<class T> std::shared_ptr<slint::VectorModel<T>> list(std::vector<T> values) {
    return std::make_shared<slint::VectorModel<T>>(std::move(values));
}

class SlintEditor final : public juce::AudioProcessorEditor, private juce::Timer {
public:
    explicit SlintEditor(FilterProcessor& processor) : AudioProcessorEditor(processor), model(processor) {
        setSize(1080, 760);
        setResizable(true, true);
        setResizeLimits(860, 680, 1800, 1200);
        setWantsKeyboardFocus(true);
        auto& platform = JuceSlintPlatform::get();
        {
            JuceSlintPlatform::Association association(platform, *this, adapter);
            ui.emplace(FilterUi::create());
        }
        (*ui)->set_nodes(nodes); (*ui)->set_meters(levels);
        (*ui)->set_knob_values(values); (*ui)->set_knob_labels(labels); (*ui)->set_knob_texts(texts);
        (*ui)->on_node_gesture([this](int band, int phase, float x, float y) {
            if (phase == 0) model.beginNode(band);
            else if (phase == 1) model.dragNode(x, y);
            else model.endNode();
            refresh();
        });
        (*ui)->on_knob_gesture([this](int knob, int phase, float next) {
            if (phase == 0) model.beginKnob(knob);
            else if (phase == 1) model.setKnob(knob, next);
            else model.endKnob();
            refresh();
        });
        (*ui)->on_toggle([this](int button) { model.toggle(button == 0 ? bypass : audition); refresh(); });
        (*ui)->show();
        resized();
        refresh();
        startTimerHz(30);
    }
    ~SlintEditor() override {
        stopTimer();
        model.endNode(); model.endKnob();
        if (ui) (*ui)->hide();
        ui.reset(); adapter = nullptr;
    }
    void paint(juce::Graphics& graphics) override {
        slint::platform::update_timers_and_animations();
        if (adapter != nullptr) adapter->paint(graphics);
    }
    void resized() override {
        if (adapter != nullptr) {
            // A fixed Window width/height binding survives dispatch_resize_event.
            // Update its explicit viewport inputs as well as the platform window size.
            (*ui)->set_viewport_width(float(getWidth()));
            (*ui)->set_viewport_height(float(getHeight()));
            adapter->window().dispatch_resize_event(slint::LogicalSize({float(getWidth()), float(getHeight())}));
        }
    }
    void mouseDown(const juce::MouseEvent& event) override {
        grabKeyboardFocus();
        slint::platform::update_timers_and_animations();
        adapter->window().dispatch_pointer_press_event(position(event), button(event));
    }
    void mouseUp(const juce::MouseEvent& event) override {
        adapter->window().dispatch_pointer_release_event(position(event), button(event));
    }
    void mouseMove(const juce::MouseEvent& event) override {
        adapter->window().dispatch_pointer_move_event(position(event));
    }
    void mouseDrag(const juce::MouseEvent& event) override { mouseMove(event); }
    void mouseExit(const juce::MouseEvent&) override { adapter->window().dispatch_pointer_exit_event(); }
    bool keyPressed(const juce::KeyPress& key) override {
        auto text = juce::String::charToString(key.getTextCharacter());
        if (key.getKeyCode() == juce::KeyPress::escapeKey) text = "\x1b";
        if (text.isEmpty()) return false;
        slint::SharedString keyText(text.toRawUTF8());
        adapter->window().dispatch_key_press_event(keyText);
        adapter->window().dispatch_key_release_event(keyText);
        return true;
    }
    void focusGained(FocusChangeType) override {
        if (adapter) adapter->window().dispatch_window_active_changed_event(true);
    }
    void focusLost(FocusChangeType) override {
        model.endNode(); model.endKnob();
        if (adapter) adapter->window().dispatch_window_active_changed_event(false);
    }
    SlintCheckGeometry geometry() const {
        const auto& view = *(*ui).operator->();
        return {{view.get_graph_left(), view.get_graph_top(), view.get_graph_width(), view.get_graph_height()},
                {view.get_frequency_knob_x(), view.get_frequency_knob_y()},
                {view.get_about_button_x(), view.get_about_button_y()},
                {view.get_about_close_x(), view.get_about_close_y()}, view.get_about_open()};
    }
private:
    static slint::LogicalPosition position(const juce::MouseEvent& event) {
        return slint::LogicalPosition({event.position.x, event.position.y});
    }
    static slint::PointerEventButton button(const juce::MouseEvent& event) {
        return event.mods.isRightButtonDown() ? slint::PointerEventButton::Right : slint::PointerEventButton::Left;
    }
    void timerCallback() override { slint::platform::update_timers_and_animations(); refresh(); }
    void refresh() {
        model.update();
        const auto& frame = model.frame();
        auto& view = *(*ui).operator->();
        view.set_response_command(pathCommand(frame.responseDb, minimumDb, maximumDb));
        view.set_input_command(pathCommand(frame.inputDb, -90, 0));
        view.set_output_command(pathCommand(frame.outputDb, -90, 0));
        for (int band = 0; band < 3; ++band)
            nodes->set_row_data(size_t(band), BandNode{frame.nodes[size_t(band)].x, frame.nodes[size_t(band)].y});
        for (int meter = 0; meter < 4; ++meter)
            levels->set_row_data(size_t(meter), juce::jlimit(0.0f, 1.0f,
                (juce::Decibels::gainToDecibels(frame.meters[size_t(meter)], -60.0f) + 60) / 60));
        for (int knob = 0; knob < 3; ++knob) {
            values->set_row_data(size_t(knob), model.knobValue(knob));
            labels->set_row_data(size_t(knob), slint::SharedString(model.knobLabel(knob).toRawUTF8()));
            texts->set_row_data(size_t(knob), slint::SharedString(model.knobText(knob).toRawUTF8()));
        }
        view.set_selected(model.selectedBand());
        view.set_bypassed(model.enabled(bypass));
        view.set_auditioning(model.enabled(audition));
        if (frame.sequence != lastSequence || lastSequence == UINT64_MAX) {
            std::vector<slint::Rgb8Pixel> pixels(size_t(historyColumns * plotBins));
            for (int x = 0; x < historyColumns; ++x) for (int y = 0; y < plotBins; ++y) {
                const auto column = (frame.historyHead + x) % historyColumns;
                const float db = frame.historyDb[size_t(column * plotBins + plotBins - 1 - y)];
                const float intensity = juce::jlimit(0.0f, 1.0f, (db + 90) / 90);
                auto colour = juce::Colour::fromHSV(.64f - .48f * intensity, .8f, .07f + .93f * intensity, 1);
                pixels[size_t(y * historyColumns + x)] = {colour.getRed(), colour.getGreen(), colour.getBlue()};
            }
            view.set_history(slint::Image(slint::SharedPixelBuffer<slint::Rgb8Pixel>(historyColumns, plotBins, pixels.data())));
            lastSequence = frame.sequence;
        }
        repaint();
    }
    FilterViewModel model;
    // Preserve repeater identity while a TouchArea owns an active drag.
    std::shared_ptr<slint::VectorModel<BandNode>> nodes = list(std::vector<BandNode>(3));
    std::shared_ptr<slint::VectorModel<float>> levels = list(std::vector<float>(4));
    std::shared_ptr<slint::VectorModel<float>> values = list(std::vector<float>(3));
    std::shared_ptr<slint::VectorModel<slint::SharedString>> labels = list(std::vector<slint::SharedString>(3));
    std::shared_ptr<slint::VectorModel<slint::SharedString>> texts = list(std::vector<slint::SharedString>(3));
    SlintWindow* adapter = nullptr;
    std::optional<slint::ComponentHandle<FilterUi>> ui;
    uint64_t lastSequence = UINT64_MAX;
};
}
juce::AudioProcessorEditor* makeFilterEditor(FilterProcessor& processor) { return new SlintEditor(processor); }
SlintCheckGeometry readSlintCheckGeometry(juce::AudioProcessorEditor& editor) {
    if (auto* slint = dynamic_cast<SlintEditor*>(&editor)) return slint->geometry();
    return {};
}
}
