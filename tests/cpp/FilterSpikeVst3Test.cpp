#include <juce_audio_utils/juce_audio_utils.h>
#include <array>
#include <cmath>
#include <iostream>
#include <set>
#include <stdexcept>
#include <X11/Xlib.h>
#include <X11/Xutil.h>

namespace {
constexpr std::array<const char*, 9> ids {
    "hp_frequency", "hp_q", "bell_frequency", "bell_q", "bell_gain",
    "lp_frequency", "lp_q", "bypass", "audition" };
using Parameters = std::array<juce::AudioProcessorParameter*, 9>;
using Stereo = std::array<std::array<float, 64>, 2>;
class EditorWindow final : public juce::DocumentWindow {
public:
    EditorWindow(juce::AudioProcessorEditor& editor, int index)
        : DocumentWindow("Filter spike VST3 instance " + juce::String(index + 1), juce::Colours::black, closeButton) {
        setUsingNativeTitleBar(true);
        setContentNonOwned(&editor, true);
        setTopLeftPosition(20 + index * 26, 30 + index * 24);
        setVisible(true);
    }
    void closeButtonPressed() override {}
};
void require(bool condition, const char* reason) {
    if (!condition) throw std::runtime_error(reason);
}
Parameters parameterContract(juce::AudioPluginInstance& plugin) {
    require(plugin.getParameters().size() == 9, "VST3 must expose exactly nine parameters, including its real bypass");
    Parameters found {};
    std::set<juce::String> observed;
    for (auto* parameter : plugin.getParameters()) {
        const auto* hosted = dynamic_cast<juce::HostedAudioProcessorParameter*>(parameter);
        require(hosted != nullptr, "VST3 parameter must expose a stable host ID");
        const auto id = hosted->getParameterID();
        require(observed.insert(id).second, "VST3 parameter IDs must be unique");
        for (size_t i = 0; i < ids.size(); ++i) {
            // VST3 hosts see the JUCE wrapper's stable numeric ID, not its original string.
            if (id == juce::String(juce::VST3ClientExtensions::convertJuceParameterId(ids[i]))) found[i] = parameter;
        }
    }
    for (auto* parameter : found) require(parameter != nullptr, "Expected stable filter parameter ID missing from real VST3");
    return found;
}
void reset(juce::AudioPluginInstance& plugin) {
    plugin.releaseResources();
    plugin.setPlayConfigDetails(2, 2, 48000, 64);
    plugin.prepareToPlay(48000, 64);
    require(plugin.getTotalNumInputChannels() == 2 && plugin.getTotalNumOutputChannels() == 2,
            "Real VST3 must accept stereo input and output");
}
Stereo impulse(juce::AudioPluginInstance& plugin) {
    juce::AudioBuffer<float> block(2, 64);
    block.clear(); block.setSample(0, 0, .5f); block.setSample(1, 0, -.25f);
    juce::MidiBuffer midi; plugin.processBlock(block, midi);
    Stereo result {};
    for (int channel = 0; channel < 2; ++channel)
        std::copy_n(block.getReadPointer(channel), 64, result[static_cast<size_t>(channel)].begin());
    return result;
}
void signedOracle(const Stereo& output) {
    const auto q = std::sqrt(.5);
    const auto omega = 2 * std::acos(-1.0) * 100 / 48000;
    const auto hp = (1 + std::cos(omega)) * .5 / (1 + std::sin(omega) / (2 * q));
    const auto lp = .25 / (1 + std::sqrt(3.0 / 8.0));
    require(std::abs(output[0][0] - .5 * hp * lp) < 1e-6 &&
            std::abs(output[1][0] + .25 * hp * lp) < 1e-6,
            "Actual VST3 signed stereo impulse differs from independent RBJ coefficient");
    for (const auto& channel : output) for (auto value : channel) require(std::isfinite(value), "VST3 output must stay finite");
}
void set(juce::AudioProcessorParameter& parameter, float normalized) {
    parameter.beginChangeGesture(); parameter.setValueNotifyingHost(normalized); parameter.endChangeGesture();
}
void pump(int milliseconds) { juce::MessageManager::getInstance()->runDispatchLoopUntil(milliseconds); }
void signal(juce::AudioPluginInstance& plugin) {
    juce::AudioBuffer<float> block(2, 64); juce::MidiBuffer midi;
    for (int iteration = 0; iteration < 48; ++iteration) {
        for (int sample = 0; sample < 64; ++sample) {
            const auto value = .2f * std::sin(2 * juce::MathConstants<double>::pi * 440 * (iteration * 64 + sample) / 48000);
            block.setSample(0, sample, static_cast<float>(value)); block.setSample(1, sample, static_cast<float>(value));
        }
        plugin.processBlock(block, midi);
        if (iteration % 8 == 0) pump(40);
    }
    pump(120);
}
juce::Image capture(juce::AudioProcessorEditor& editor, const juce::File& file) {
    require(editor.getWidth() >= 900 && editor.getHeight() >= 600, "VST3 custom editor has invalid dimensions");
    // A VST3 editor is a host proxy containing a separate native plugin child.
    // Capture only this task-owned X11 peer; repainting the proxy omits that child.
    auto* owner = editor.getTopLevelComponent();
    owner->toFront(false); pump(150);
    auto* peer = owner->getPeer(); require(peer != nullptr, "VST3 editor needs its task-owned native peer");
    const auto origin = owner->getLocalPoint(&editor, juce::Point<int>());
    std::unique_ptr<Display, decltype(&XCloseDisplay)> display(XOpenDisplay(nullptr), &XCloseDisplay);
    require(display != nullptr, "Linux VST3 capture needs an X11 display");
    const auto window = static_cast<Window>(reinterpret_cast<std::uintptr_t>(peer->getNativeHandle()));
    XWindowAttributes attributes {};
    require(XGetWindowAttributes(display.get(), window, &attributes) != 0 && attributes.visual->c_class == TrueColor,
            "Task-owned VST3 peer must use an X11 TrueColor visual");
    require(attributes.width >= origin.x + editor.getWidth() && attributes.height >= origin.y + editor.getHeight(),
            "Native VST3 peer must cover the requested editor extent");
    XSync(display.get(), False);
    auto destroyImage = [](XImage* pixels) { if (pixels != nullptr) XDestroyImage(pixels); };
    std::unique_ptr<XImage, decltype(destroyImage)> pixels(
        XGetImage(display.get(), window, origin.x, origin.y, static_cast<unsigned int>(editor.getWidth()),
                  static_cast<unsigned int>(editor.getHeight()), AllPlanes, ZPixmap), destroyImage);
    require(pixels != nullptr, "Cannot read task-owned native VST3 child pixels");
    auto channel = [](unsigned long pixel, unsigned long mask) {
        if (mask == 0) return static_cast<juce::uint8>(0);
        unsigned int shift = 0;
        while (((mask >> shift) & 1UL) == 0) ++shift;
        return static_cast<juce::uint8>(((pixel & mask) >> shift) * 255UL / (mask >> shift));
    };
    juce::Image image(juce::Image::RGB, editor.getWidth(), editor.getHeight(), false);
    juce::Image::BitmapData destination(image, juce::Image::BitmapData::writeOnly);
    for (int y = 0; y < image.getHeight(); ++y) for (int x = 0; x < image.getWidth(); ++x) {
        const auto pixel = XGetPixel(pixels.get(), x, y);
        destination.setPixelColour(x, y, juce::Colour(channel(pixel, pixels->red_mask), channel(pixel, pixels->green_mask), channel(pixel, pixels->blue_mask)));
    }
    std::set<juce::uint32> colours;
    for (int y = 0; y < image.getHeight(); y += 3) for (int x = 0; x < image.getWidth(); x += 3)
        colours.insert(image.getPixelAt(x, y).getARGB());
    auto stream = file.createOutputStream();
    require(stream != nullptr, "Cannot open VST3 editor capture");
    require(stream->setPosition(0) && stream->truncate().wasOk(), "Cannot replace VST3 editor capture");
    require(juce::PNGImageFormat().writeImageToStream(image, *stream), "Cannot save VST3 editor capture");
    stream->flush();
    Window rootWindow {}, parentWindow {}; Window* children = nullptr; unsigned int childCount = 0;
    XQueryTree(display.get(), window, &rootWindow, &parentWindow, &children, &childCount);
    if (children != nullptr) XFree(children);
    std::cout << "CAPTURE file=" << file.getFullPathName() << " colours=" << colours.size()
              << " native=" << window << " crop=" << origin.x << ',' << origin.y << ',' << editor.getWidth() << ',' << editor.getHeight()
              << " showing=" << editor.isShowing() << " nativeChildren=" << childCount
              << " rgbMasks=" << pixels->red_mask << ',' << pixels->green_mask << ',' << pixels->blue_mask << std::endl;
    if (const auto* pause = std::getenv("DANDRUM_FILTER_CAPTURE_PAUSE_MS")) pump(juce::jlimit(0, 5000, std::atoi(pause)));
    require(colours.size() > 16, "Actual VST3 editor snapshot must contain rendered graphics");
    return image;
}
bool different(const juce::Image& first, const juce::Image& second) {
    // Both comparison layouts put rotary controls in the bottom fifth. Exclude
    // animated spectrum/history/meters so motion cannot impersonate host polling.
    for (int y = first.getHeight() * 4 / 5; y < first.getHeight(); y += 3) for (int x = 0; x < first.getWidth(); x += 3)
        if (first.getPixelAt(x, y) != second.getPixelAt(x, y)) return true;
    return false;
}
}

int main(int argc, char** argv) {
    try {
        require(argc == 3, "Usage: filter-spike-vst3-test <plugin.vst3> <capture-directory>");
        juce::ScopedJuceInitialiser_GUI initialise;
        // Establish JUCE's native desktop peer before loading a plugin's separately linked GUI runtime.
        juce::DocumentWindow desktop("Filter spike VST3 host", juce::Colours::darkgrey, 0);
        desktop.setUsingNativeTitleBar(true);
        desktop.setBounds(0, 0, 1200, 900);
        desktop.setVisible(true); pump(50);
        juce::VST3PluginFormat format;
        juce::OwnedArray<juce::PluginDescription> descriptions;
        format.findAllTypesForFile(descriptions, juce::File(argv[1]).getFullPathName());
        require(descriptions.size() == 1, "Expected exactly one effect in real VST3 bundle");
        juce::AudioPluginFormatManager manager;
        manager.addFormat(new juce::VST3PluginFormat()); // JUCE 8.0.6 takes ownership.
        juce::String error;
        auto first = manager.createPluginInstance(*descriptions[0], 48000, 64, error);
        require(first != nullptr, ("Cannot create first VST3 instance: " + error).toRawUTF8());
        auto second = manager.createPluginInstance(*descriptions[0], 48000, 64, error);
        require(second != nullptr, ("Cannot create second VST3 instance: " + error).toRawUTF8());
        auto parameters = parameterContract(*first), other = parameterContract(*second);
        reset(*first); reset(*second);
        signedOracle(impulse(*first)); signedOracle(impulse(*second));
        set(*parameters[7], 1.f);
        auto dry = impulse(*first);
        for (size_t channel = 0; channel < 2; ++channel) for (size_t sample = 0; sample < 64; ++sample)
            require(dry[channel][sample] == (sample == 0 ? (channel == 0 ? .5f : -.25f) : 0.f), "VST3 bypass must preserve exact signed input");
        set(*parameters[7], 0.f); set(*parameters[2], .71f); set(*parameters[4], .66f);
        juce::MemoryBlock state; first->getStateInformation(state);
        second->setStateInformation(state.getData(), static_cast<int>(state.getSize()));
        reset(*first); reset(*second);
        auto restoredFirst = impulse(*first), restoredSecond = impulse(*second);
        for (size_t channel = 0; channel < 2; ++channel) for (size_t sample = 0; sample < 64; ++sample)
            require(std::abs(restoredFirst[channel][sample] - restoredSecond[channel][sample]) < 1e-7,
                    "Saved VST3 state must restore identical stereo filter output");
        auto output = juce::File(argv[2]); require(output.createDirectory().wasOk(), "Cannot create capture directory");
        std::unique_ptr<juce::AudioProcessorEditor> editor(first->createEditorIfNeeded());
        std::unique_ptr<juce::AudioProcessorEditor> otherEditor(second->createEditorIfNeeded());
        require(editor != nullptr && otherEditor != nullptr, "Both VST3 instances must create custom editors");
        auto window = std::make_unique<EditorWindow>(*editor, 0);
        auto otherWindow = std::make_unique<EditorWindow>(*otherEditor, 1);
        signal(*first); signal(*second);
        auto before = capture(*editor, output.getChildFile("first.png"));
        capture(*otherEditor, output.getChildFile("second.png"));
        const auto untouched = other[2]->getValue();
        set(*parameters[2], .87f); signal(*first);
        auto after = capture(*editor, output.getChildFile("host-update.png"));
        require(std::abs(parameters[2]->getValue() - .87f) < 1e-6 && different(before, after),
                "Host parameter change must appear in actual VST3 rotary-control pixels");
        require(std::abs(other[2]->getValue() - untouched) < 1e-6, "Simultaneous VST3 instances must retain independent parameter state");
        const auto kept = parameters[2]->getValue();
        window.reset(); editor.reset(); pump(60);
        editor.reset(first->createEditorIfNeeded()); require(editor != nullptr, "VST3 editor must reopen");
        window = std::make_unique<EditorWindow>(*editor, 0); signal(*first);
        capture(*editor, output.getChildFile("reopened.png"));
        require(std::abs(parameters[2]->getValue() - kept) < 1e-6 && std::abs(other[2]->getValue() - untouched) < 1e-6,
                "VST3 editor recreation must preserve both instances' host parameters");
        editor->setSize(1280, 900); pump(100);
        capture(*editor, output.getChildFile("resized.png"));
        window.reset(); otherWindow.reset(); editor.reset(); otherEditor.reset(); first->releaseResources(); second->releaseResources();
        std::cout << descriptions[0]->name << ": real VST3 nine-ID/stereo/signed oracle/bypass/state/two-editor/host-update/reopen checks passed\n";
        return 0;
    } catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
