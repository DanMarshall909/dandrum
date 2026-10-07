#pragma once
#include <juce_gui_basics/juce_gui_basics.h>
#include <slint-platform.h>
#include <stdexcept>
#include <vector>

namespace filter_spike {
class SlintWindow final : public slint::platform::WindowAdapter {
public:
    explicit SlintWindow(juce::Component& owner) : component(&owner),
        software(slint::platform::SoftwareRenderer::RepaintBufferType::NewBuffer) {}
    slint::PhysicalSize size() override {
        if (component == nullptr) return slint::PhysicalSize({1, 1});
        return slint::PhysicalSize({uint32_t(component->getWidth()), uint32_t(component->getHeight())});
    }
    slint::platform::AbstractRenderer& renderer() override { return software; }
    void request_redraw() override { if (component != nullptr) component->repaint(); }
    void set_visible(bool visible) override {
        if (visible) window().dispatch_scale_factor_change_event(1.0f);
    }
    void set_size(slint::PhysicalSize next) override {
        if (component != nullptr) component->setSize(int(next.width), int(next.height));
    }
    void paint(juce::Graphics& graphics) {
        const auto dimensions = size();
        if (dimensions.width == 0 || dimensions.height == 0) return;
        pixels.resize(size_t(dimensions.width) * dimensions.height);
        software.render(std::span<slint::Rgb8Pixel>(pixels), dimensions.width);
        if (image.getWidth() != int(dimensions.width) || image.getHeight() != int(dimensions.height))
            image = juce::Image(juce::Image::RGB, int(dimensions.width), int(dimensions.height), false);
        {
            juce::Image::BitmapData destination(image, juce::Image::BitmapData::writeOnly);
            for (int y = 0; y < destination.height; ++y) {
                auto* row = reinterpret_cast<juce::PixelRGB*>(destination.getLinePointer(y));
                const auto* source = pixels.data() + size_t(y) * dimensions.width;
                for (int x = 0; x < destination.width; ++x)
                    row[x].setARGB(255, source[x].r, source[x].g, source[x].b);
            }
        }
        graphics.drawImageAt(image, 0, 0);
    }
private:
    juce::Component::SafePointer<juce::Component> component;
    slint::platform::SoftwareRenderer software;
    std::vector<slint::Rgb8Pixel> pixels;
    juce::Image image;
};

class JuceSlintPlatform final : public slint::platform::Platform {
public:
    std::unique_ptr<slint::platform::WindowAdapter> create_window_adapter() override {
        // Each generated component is constructed synchronously inside Association.
        if (pending == nullptr || result == nullptr) throw std::logic_error("Unassociated Slint window");
        auto adapter = std::make_unique<SlintWindow>(*pending);
        *result = adapter.get();
        return adapter;
    }
    void run_in_event_loop(Task task) override {
        auto shared = std::make_shared<Task>(std::move(task));
        juce::MessageManager::callAsync([shared] { std::move(*shared).run(); });
    }
    void set_clipboard_text(const slint::SharedString& text, Clipboard) override {
        juce::SystemClipboard::copyTextToClipboard(juce::String::fromUTF8(text.data()));
    }
    std::optional<slint::SharedString> clipboard_text(Clipboard) override {
        return slint::SharedString(juce::SystemClipboard::getTextFromClipboard().toRawUTF8());
    }
    class Association {
    public:
        Association(JuceSlintPlatform& host, juce::Component& owner, SlintWindow*& target)
            : platform(host), previous(host.pending), previousResult(host.result) {
            host.pending = &owner; host.result = &target;
        }
        ~Association() { platform.pending = previous; platform.result = previousResult; }
    private:
        JuceSlintPlatform& platform;
        juce::Component* previous;
        SlintWindow** previousResult;
    };
    static JuceSlintPlatform& get() {
        // First construction and all subsequent UI work belong to JUCE's message thread.
        jassert(juce::MessageManager::getInstance()->isThisTheMessageThread());
        static auto* instance = [] {
            auto platform = std::make_unique<JuceSlintPlatform>();
            auto* raw = platform.get();
            slint::platform::set_platform(std::move(platform));
            return raw;
        }();
        return *instance;
    }
private:
    juce::Component* pending = nullptr;
    SlintWindow** result = nullptr;
};
}
