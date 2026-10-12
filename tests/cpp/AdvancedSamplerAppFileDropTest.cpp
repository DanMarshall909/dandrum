#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/engine/FileOperations.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/advanced-sampler/platform/WindowFileDrop.h"
#include "ui/advanced-sampler/theme/ThemeBinding.h"
#include "ui/slint/host/ValueCodec.h"
#include "tests/fixtures/advanced_sampler_xdnd_producer.h"
#include <slint-testing.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>

namespace {
constexpr const char* title = "Dandrum Advanced Sampler · silent editor";
using Clock = std::chrono::steady_clock;
void writeWav(const std::filesystem::path& path) {
    std::ofstream file(path, std::ios::binary);
    auto u16 = [&](unsigned value) { file.put(value & 255); file.put((value >> 8) & 255); };
    auto u32 = [&](unsigned value) { u16(value & 65535); u16(value >> 16); };
    file.write("RIFF", 4); u32(44); file.write("WAVEfmt ", 8); u32(16);
    u16(1); u16(1); u32(4); u32(8); u16(2); u16(16);
    file.write("data", 4); u32(8); u16(0); u16(16384); u16(32768); u16(32767);
    if (!file) throw std::runtime_error("Cannot create the native drop WAV fixture");
}
std::string uri(const std::filesystem::path& path, bool localhost = false) {
    std::string result = localhost ? "file://localhost" : "file://";
    constexpr char digits[] = "0123456789ABCDEF";
    for (const unsigned char c : path.native()) {
        if (c == ' ' || c >= 128 || c == '%' || c == '#' || c == '?') {
            result += '%'; result += digits[c >> 4]; result += digits[c & 15];
        } else result += c;
    }
    return result;
}
}

// The actual maintained AppWindow and production file/session bindings run on
// the real winit backend. A separate X11 producer delivers files; pointer events
// choose the actual mapping dialog and Undo. No receive_files call occurs here.
int main(int argc, char** argv) {
    int checks = 0;
    const auto require = [&](bool value, const std::string& message) {
        ++checks; if (!value) throw std::runtime_error(message);
    };
    try {
        const auto directory = argc > 1 ? std::filesystem::absolute(argv[1]) : std::filesystem::temp_directory_path();
        const auto single = directory / "Native C3 音 sample.wav";
        const auto soft = directory / "Felt_C3_p.wav";
        const auto hard = directory / "Felt_C3_f.wav";
        for (const auto& path : {single, soft, hard}) writeWav(path);
        auto window = dandrum_ui::AppWindow::create();
        auto model = std::make_shared<dandrum::sampler::Model>();
        require(model->command("patch.new"), "Begin the maintained application with an empty instrument");
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window, model);
        dandrum::sampler::bind_file_operations(window, binding);
        dandrum::sampler::bind_performance_sources(window->global<dandrum_ui::PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<dandrum_ui::PatchFilter>());
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        dandrum::advanced_sampler::apply_theme(window->global<dandrum_ui::SamplerTheme>(),
                dandrum::advanced_sampler::resolve_theme({}));
        // Main retains this identical production owner after bind_file_operations.
        auto desktopDrop = dandrum::sampler::bind_native_file_drop(window);
        window->invoke_choose_size(1);
        const auto elements = [&](const std::string& label) {
            std::vector<slint::testing::ElementHandle> result;
            for (auto element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.size().width > 0 && element.size().height > 0) result.push_back(element);
            return result;
        };
        const auto element = [&](const std::string& label) {
            auto found = elements(label);
            if (found.empty()) throw std::runtime_error("Missing actual native drop control: " + label);
            for (auto candidate : found)
                if (candidate.accessible_role() == slint::language::AccessibleRole::Button ||
                    candidate.accessible_role() == slint::language::AccessibleRole::TextInput) return candidate;
            return found.back();
        };
        const auto click = [&](const std::string& label) {
            auto control = element(label); const auto at = control.absolute_position(); const auto size = control.size();
            slint::LogicalPosition point{{at.x + size.width / 2, at.y + size.height / 2}};
            window->window().dispatch_pointer_press_event(point, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(point, slint::PointerEventButton::Left);
        };
        struct Case { std::string selection, mode, label; bool invalid = false; };
        const std::string multiple = "# genuine multiple local files\r\n" + uri(soft) + "\r\n" + uri(hard, true) + "\n";
        const std::vector<Case> cases{
            {uri(single), "single", ""},
            {multiple, "sequential", "One per key"},
            {multiple, "root-velocity", "By root + velocity"},
            {multiple, "stack", "Stack"},
            {multiple, "round-robin", "Round robin"},
            {"file://foreign-host" + soft.native(), "invalid", "", true},
            {uri(soft) + "\nfile:///tmp/invalid%00.wav", "invalid", "", true},
        };
        std::size_t current = 0;
        auto source = std::make_unique<dandrum::test::NativeXdndProducer>(title, cases.front().selection);
        auto began = Clock::now();
        bool imported = false;
        std::string failure;
        slint::Timer test;
        test.start(slint::TimerMode::Repeated, std::chrono::milliseconds(20), [&] {
            try {
                const auto& item = cases[current];
                if (Clock::now() - began > std::chrono::seconds(8)) throw std::runtime_error("Native composed drop timed out in " + item.mode);
                if (!source->targetHasProxy() && Clock::now() - began < std::chrono::milliseconds(1500)) return;
                source->poll();
                if (source->hovering())
                    require(model->records("source").empty() && model->records("zone").empty() && model->undoCount() == 0,
                            "Real desktop hover does not import a WAV or create history before Drop");
                if (!source->finished()) return;
                require(source->exchanged(), "Actual composed application completes a genuine XDND wire exchange");
                require(source->delivered() != item.invalid, "Actual composed application reports valid local drops and rejects invalid URI selections");
                if (item.invalid) {
                    require(model->records("source").empty() && model->records("zone").empty() && model->undoCount() == 0 && elements("File paths").empty(),
                            "Invalid real URI drop leaves the maintained model, Undo and import chooser unchanged");
                } else {
                    if (!imported && item.mode != "single") {
                        require(model->records("source").empty() && model->records("zone").empty() && model->undoCount() == 0,
                                "Actual multi-file desktop drop opens the chooser before any native import mutation");
                        require(element("File paths").accessible_value() == slint::SharedString(soft.native() + "\n" + hard.native()),
                                "Actual chooser preserves the exact ordered paths decoded from real XDND");
                        require(element("By root + velocity").accessible_checked() == true,
                                "Actual desktop multi-drop defaults to the guide root-and-velocity choice");
                        click(item.label);
                        require(element(item.label).accessible_checked() == true,
                                "Actual pointer chooses the requested desktop mapping mode: " + item.mode);
                        click("Continue");
                        require(elements("File paths").empty(), "Actual Continue closes the desktop import chooser after production file binding accepts it");
                    }
                    imported = true;
                    const auto count = item.mode == "single" ? 1u : 2u;
                    require(model->records("source").size() == count && model->records("zone").size() == count && model->undoCount() == 1,
                            "Real desktop drop reaches maintained AppWindow file callbacks and Model as one import transaction: " + item.mode);
                    if (std::ranges::any_of(model->records("source"), [](const auto& record) { return record.text("state") == "loading"; })) return;
                    for (const auto& record : model->records("source"))
                        require(record.text("state") == "loaded" && record.number("frames") == 4 && record.number("channels") == 1 && record.number("sample-rate") == 4,
                                "Production UI timer reads the genuine WAV file and projects its actual sample metadata");
                    if (item.mode == "single") {
                        require(model->records("source").front().text("path") == single.native() && model->text("selected-source") == model->records("source").front().id,
                                "Single real desktop drop preserves its exact Unicode/spaces path and selects its imported source");
                        require(model->peaks().size() == 4 && model->peaks()[2].minimum == -1.f && model->peaks()[3].maximum == 32767.f / 32768.f,
                                "Single desktop WAV import publishes its known signed waveform values");
                    } else {
                        require(model->records("source")[0].text("path") == soft.native() && model->records("source")[1].text("path") == hard.native(),
                                "Production multi-import preserves actual XDND source order and paths");
                        const auto& a = model->records("zone")[0]; const auto& b = model->records("zone")[1];
                        if (item.mode == "sequential") require(a.number("lo") == 48 && a.number("hi") == 48 && b.number("lo") == 49 && b.number("hi") == 49,
                                "Actual desktop One per key choice creates consecutive C3 mappings");
                        else if (item.mode == "root-velocity") require(a.number("root-note") == 48 && b.number("root-note") == 48 && a.number("lo") == 0 && a.number("hi") == 127 && a.number("vel-hi") == 90 && b.number("vel-lo") == 81,
                                "Actual desktop root-and-velocity choice reads C3 soft/hard filenames and fills mapped keys");
                        else if (item.mode == "stack") require(a.number("lo") == 0 && a.number("hi") == 127 && b.number("lo") == 0 && b.number("hi") == 127 && model->text("selector-mode") == "stack",
                                "Actual desktop Stack choice maps both sources across the keyboard");
                        else require(a.number("lo") == 48 && a.number("hi") == 48 && b.number("lo") == 48 && b.number("hi") == 48 && model->text("selector-mode") == "round-robin",
                                "Actual desktop Round robin choice maps both sources to C3 and enables alternating selection");
                    }
                    click("Undo");
                    require(model->records("source").empty() && model->records("zone").empty() && model->undoCount() == 0,
                            "Actual application Undo removes the complete single or multi-file desktop import");
                }
                std::cout << "Verified genuine maintained AppWindow XDND: " << item.mode << '\n';
                if (++current == cases.size()) { test.stop(); slint::quit_event_loop(); }
                else {
                    require(model->command("patch.new"), "Reset the next composed desktop drop to an empty instrument"); binding->refresh();
                    source = std::make_unique<dandrum::test::NativeXdndProducer>(title, cases[current].selection);
                    began = Clock::now(); imported = false;
                }
            } catch (const std::exception& error) { failure = error.what(); test.stop(); slint::quit_event_loop(); }
        });
        window->run();
        if (!failure.empty()) throw std::runtime_error(failure);
        require(current == cases.size(), "Every actual maintained-application desktop drop case completes");
        std::cout << "PASS: " << checks << " actual AppWindow desktop-file-drop checks across " << current << " real XDND exchanges\n";
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
