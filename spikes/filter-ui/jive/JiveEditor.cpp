#include "ResponseGraph.h"
#include "AnalysisPanels.h"
#include "RotaryControl.h"
#include <jive_layouts/jive_layouts.h>
#include <jive_style_sheets/jive_style_sheets.h>

namespace filter_spike::jive_ui {
class FilterView final : public jive::View, private juce::Timer {
public:
    explicit FilterView(FilterProcessor* processor) : model(std::make_shared<FilterViewModel>(*processor)) {}
    ~FilterView() override { stopTimer(); }
private:
    juce::ValueTree initialise() override {
        auto style = new jive::Object {
            { "background", "#11151c" }, { "foreground", "#eef4ff" },
            { "font-size", 12 }, { "font-family", "DejaVu Sans" },
            { "Button", new jive::Object {
                { "background", "#222c3b" }, { "foreground", "#dbe5f5" },
                { "border-radius", 8 }, { "hover", new jive::Object { { "background", "#34455b" } } }
            } },
            { "#title", new jive::Object { { "font-size", 24 }, { "font-style", "bold" } } },
            { ".subtle", new jive::Object { { "foreground", "#8b9bb1" } } }
        };
        juce::ValueTree controls { "Component", {{ "flex-direction", "row" }, { "height", 136 }, { "gap", 24 }, { "align-items", "centre" }} };
        for (int i = 0; i < 3; ++i) {
            knobLabels[static_cast<size_t>(i)] = { "Text", {{ "text", "" }, { "height", 20 }, { "width", 180 }, { "justification", "centred" }} };
            knobValues[static_cast<size_t>(i)] = { "Text", {{ "text", "" }, { "height", 20 }, { "width", 180 }, { "justification", "centred" }} };
            controls.appendChild({ "Component", {{ "width", 180 }, { "height", 136 }, { "align-items", "centre" }}, {
                knobLabels[static_cast<size_t>(i)],
                juce::ValueTree { "Knob", {{ "id", "knob" + juce::String(i) }, { "control", i }, { "min", 0.0 }, { "max", 1.0 }, { "value", .5 }, { "width", 88 }, { "height", 88 }} },
                knobValues[static_cast<size_t>(i)] } }, nullptr);
        }
        juce::ValueTree bands { "Component", {{ "width", 290 }, { "height", 136 }, { "justify-content", "centre" }, { "gap", 8 }}, {
            juce::ValueTree { "Text", {{ "text", "SELECT BAND" }, { "height", 20 }, { "class", "subtle" }} },
            juce::ValueTree { "Component", {{ "flex-direction", "row" }, { "height", 40 }, { "gap", 8 }}, {
                button("High-pass", "hp", 80, 0), button("Bell", "bell", 80, 1), button("Low-pass", "lp", 80, 2)
            } },
            juce::ValueTree { "Text", {{ "text", "Drag a graph node or turn a control." }, { "height", 24 }, { "class", "subtle" }} }
        } };
        controls.appendChild(bands, nullptr);
        juce::ValueTree tree { "Editor", {{ "width", 1080 }, { "height", 760 }, { "padding", 24 }, { "gap", 14 }, { "style", style }}, {
            juce::ValueTree { "Component", {{ "flex-direction", "row" }, { "height", 44 }, { "align-items", "centre" }, { "gap", 12 }}, {
                juce::ValueTree { "Text", {{ "id", "title" }, { "text", "DANDRUM  /  FILTER LAB" }, { "flex-grow", 1 }, { "height", 40 }} },
                button("Bypass", "bypass", 100, -1, filter_spike::bypass),
                button("Audition", "audition", 100, -1, filter_spike::audition)
            } },
            juce::ValueTree { "Text", {{ "text", "JIVE  /  native declarative editor  /  stereo HP -> bell -> LP" }, { "height", 20 }, { "class", "subtle" }} },
            juce::ValueTree { "ResponseGraph", {{ "height", 300 }, { "width", "100%" }} },
            juce::ValueTree { "Component", {{ "flex-direction", "row" }, { "height", 140 }, { "gap", 14 }}, {
                juce::ValueTree { "Spectrogram", {{ "flex-grow", 1 }, { "height", 140 }} },
                juce::ValueTree { "Meters", {{ "width", 128 }, { "height", 140 }} }
            } }, controls
        } };
        applySpacing(tree);
        return tree;
    }
    static void applySpacing(juce::ValueTree tree) {
        const auto gap = static_cast<int>(tree["gap"]);
        const auto horizontal = tree["flex-direction"].toString() == "row";
        tree.removeProperty("gap", nullptr);
        for (int i = 0; i < tree.getNumChildren(); ++i) {
            auto child = tree.getChild(i);
            if (i > 0 && gap > 0) child.setProperty("margin", horizontal ? "0 0 0 " + juce::String(gap) : juce::String(gap) + " 0 0 0", nullptr);
            applySpacing(child);
        }
    }
    static juce::ValueTree button(const char* text, const char* id, int width, int band, int parameter = -1) {
        return { "Button", {{ "id", id }, { "width", width }, { "height", 36 }, { "band", band }, { "parameter", parameter }},
                 { juce::ValueTree { "Text", {{ "text", text }} } } };
    }
    std::unique_ptr<juce::Component> createComponent(const juce::ValueTree& tree) override {
        const auto type = tree.getType().toString();
        if (type == "ResponseGraph") return std::make_unique<ResponseGraph>(model);
        if (type == "Spectrogram") return std::make_unique<Spectrogram>(model);
        if (type == "Meters") return std::make_unique<Meters>(model);
        if (type == "Knob") {
            auto index = static_cast<int>(tree["control"]);
            auto knob = std::make_unique<RotaryControl>(model, index);
            knobs[static_cast<size_t>(index)] = knob.get();
            return knob;
        }
        if (type == "Button") {
            auto control = std::make_unique<juce::TextButton>();
            const auto band = static_cast<int>(tree["band"]), parameter = static_cast<int>(tree["parameter"]);
            control->onClick = [source = model, band, parameter] { if (band >= 0) source->selectBand(band); else source->toggle(parameter); };
            if (parameter >= 0) toggles[static_cast<size_t>(parameter - filter_spike::bypass)] = control.get();
            else bandButtons[static_cast<size_t>(band)] = control.get();
            return control;
        }
        return nullptr;
    }
    void setup(jive::GuiItem& item) override { root = item.getComponent().get(); timerCallback(); startTimerHz(30); }
    void timerCallback() override {
        model->update();
        for (int i = 0; i < 3; ++i) {
            if (knobs[static_cast<size_t>(i)]) knobs[static_cast<size_t>(i)]->refresh();
            knobLabels[static_cast<size_t>(i)].setProperty("text", model->knobLabel(i), nullptr);
            knobValues[static_cast<size_t>(i)].setProperty("text", model->knobText(i), nullptr);
            if (bandButtons[static_cast<size_t>(i)]) bandButtons[static_cast<size_t>(i)]->setToggleState(i == model->selectedBand(), juce::dontSendNotification);
        }
        for (int i = 0; i < 2; ++i) if (toggles[static_cast<size_t>(i)]) toggles[static_cast<size_t>(i)]->setToggleState(model->enabled(filter_spike::bypass + i), juce::dontSendNotification);
        if (root) root->repaint();
    }
    Model model;
    juce::Component::SafePointer<juce::Component> root;
    std::array<juce::Component::SafePointer<RotaryControl>, 3> knobs;
    std::array<juce::Component::SafePointer<juce::TextButton>, 3> bandButtons;
    std::array<juce::Component::SafePointer<juce::TextButton>, 2> toggles;
    std::array<juce::ValueTree, 3> knobLabels, knobValues;
};
}

namespace filter_spike {
juce::AudioProcessorEditor* makeFilterEditor(FilterProcessor& processor) {
    // The View and its timer/model are owned by this interpreted editor, never by a global.
    jive::Interpreter interpreter;
    auto item = interpreter.interpret(jive::makeView<jive_ui::FilterView>(&processor), &processor);
    auto* editor = dynamic_cast<juce::AudioProcessorEditor*>(item.get());
    if (editor != nullptr) {
        editor->setResizable(true, true);
        editor->setResizeLimits(900, 700, 1800, 1200);
        // Interpretation constructs PluginEditor before its children finish layout.
        // Publish the declared initial editor extent only after that setup is complete.
        editor->setSize(1080, 760);
        item.release();
    }
    return editor;
}
}
