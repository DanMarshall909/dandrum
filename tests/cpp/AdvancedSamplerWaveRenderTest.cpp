#include "AdvancedSamplerWaveRender.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <chrono>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <vector>

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    int checks = 0;
    try {
        auto window = dandrum_ui::AdvancedSamplerWaveRenderTest::create();
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
        const auto& theme = window->global<dandrum_ui::SamplerTheme>();
        // Signed-pixel calibration uses the advertised flat well finish.
        theme.set_soft(false);
        const auto color = [](unsigned rgb) { return slint::Color::from_argb_uint8(255, rgb >> 16, rgb >> 8, rgb); };
        theme.set_ink_0(color(0x130f0c)); theme.set_ink_5(color(0x41362c));
        theme.set_accent_lo(color(0xb0662f)); theme.set_accent(color(0xe08a4e));
        theme.set_paper_1(color(0xf2e6d3)); theme.set_paper_4(color(0x909090)); theme.set_waveform_region(color(0xd0d0d0));
        const auto require = [&](bool ok, const std::string& text) { ++checks; if (!ok) throw std::runtime_error(text); };
        const auto activate = [&](std::string_view label) {
            for (auto element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.accessible_role() == slint::language::AccessibleRole::Button) {
                    element.invoke_accessible_default_action(); return;
                }
            throw std::runtime_error("Missing native display control: " + std::string(label));
        };
        // One bucket per physical pixel reproduces the maintained source view.
        // Known signed bounds must occupy its lower area, even for thin buckets.
        const auto sparse = window->get_test_peaks();
        window->set_test_peaks(std::make_shared<slint::VectorModel<dandrum_ui::WaveBucket>>(
            std::vector<dandrum_ui::WaveBucket>(600, dandrum_ui::WaveBucket{-1.f, .25f})));
        int phase = -1;
        std::string failure;
        slint::Timer timer;
        timer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(150), [&] {
            try {
                const auto image = window->window().take_snapshot();
                require(image.has_value(), "Real native renderer supplies waveform and spectral pixels");
                std::ofstream output(std::string(argv[1]) + "/wave-" + std::to_string(phase) + ".pam", std::ios::binary);
                output << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height()
                       << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                output.write(reinterpret_cast<const char*>(image->begin()), image->width() * image->height() * 4);
                const auto pixel = [&](int x, int y) { return image->begin()[y * image->width() + x]; };
                if (phase == 15) {
                    const auto metadata=slint::testing::ElementHandle::find_by_accessible_label(window,"Fixture sample.wav");
                    const auto names=slint::testing::ElementHandle::find_by_accessible_label(window,"Slice one");
                    require(metadata.size()==1 && names.size()==1,
                            "Source metadata and genuine slice labels both remain visible");
                    require(metadata[0].absolute_position().y>=names[0].absolute_position().y+names[0].size().height+2,
                            "Source metadata occupies a separate row below slice labels");
                } else if (phase == 14) {
                    const auto loaded=pixel(100,120),tail=pixel(800,120);
                    require(loaded.r>=207 && loaded.r<=208 && loaded.g>=207 && loaded.g<=208,
                            "Loading preserves the independently calibrated signed envelope in decoded columns");
                    require(tail.r==19 && tail.g==15 && tail.b==12,
                            "Loading leaves the undecoded half empty instead of painting the cached tail");
                } else if (phase == 13) {
                    const auto frames=slint::testing::ElementHandle::find_by_accessible_label(window,"Visible waveform viewport");
                    require(frames.size()==1 && std::abs(frames[0].size().width-300)<1,
                            "Two-times waveform zoom outlines exactly half of the 600-pixel source overview");
                } else if (phase == 12) {
                    const auto outline=pixel(160,220);
                    require(outline.r==144 && outline.g==144 && outline.b==144,
                            "Source overview shows its visible viewport outline at the calibrated top edge");
                    const auto mini=pixel(100,230);
                    require(mini.r>=207 && mini.r<=208 && mini.g>=207 && mini.g<=208,
                            "Miniature overview uses its full height for the calibrated signed source waveform");
                    for(const auto label:{"START","END"})
                        for(const auto& text:slint::testing::ElementHandle::find_by_accessible_label(window,label))
                            require(text.absolute_position().y<220 || text.absolute_position().y>=248,
                                    "Miniature overview excludes main-editor endpoint labels");
                } else if (phase == 11) {
                    for (int x=20;x<920;++x) {
                        const auto filled=pixel(x,155);
                        // The software renderer rounds composited edge coverage by
                        // one byte. The original seams lose 40–90 bytes here; keep
                        // the signed interior calibration exact in the other phases.
                        require(filled.r>=207 && filled.r<=208 && filled.g>=207 && filled.g<=208
                                && filled.b>=207 && filled.b<=208,
                                "Expanded variable signed waveform is continuous at every device column: " + std::to_string(x));
                    }
                } else if (phase == -1) {
                    const auto above = pixel(85, 80), negative = pixel(85, 155);
                    require(above.r == 19 && above.g == 15 && above.b == 12,
                            "Dense 600-bucket signed maximum .25 leaves its calibrated upper region empty");
                    require(negative.r == 208 && negative.g == 208 && negative.b == 208,
                            "Dense 600-bucket signed minimum -1 fills its calibrated lower region instead of collapsing to a flatline");
                } else if (phase == 8) {
                    const auto retained = pixel(85, 155);
                    require(retained.r == 208 && retained.g == 208 && retained.b == 208,
                            "Rejected unsupported replacement retains the independently calibrated previous signed waveform");
                } else if (phase == 9 || phase == 10) {
                    const auto empty = pixel(85, 155);
                    require(empty.r == 19 && empty.g == 15 && empty.b == 12,
                            "Unavailable source has no painted waveform when unsupported without cache or missing with cache");
                    const auto label = phase == 9 ? "Unsupported sample format" : "Sample unavailable · Relink or replace";
                    require(!slint::testing::ElementHandle::find_by_accessible_label(window,label).empty(),
                            "Unavailable source keeps its visible actionable waveform feedback");
                } else if (phase == 0 || phase == 4 || phase == 7) {
                    const auto above = pixel(85, 80), negative = pixel(85, 155);
                    require(above.r == 19 && above.g == 15 && above.b == 12,
                            "Signed maximum .25 leaves its known upper waveform region empty");
                    require(negative.r == 208 && negative.g == 208 && negative.b == 208,
                            "Signed minimum -1 fills its known lower waveform region");
                } else if (phase == 5) {
                    const auto first = pixel(85, 155), last = pixel(535, 155);
                    require(first.r == 19 && first.g == 15 && first.b == 12,
                            "Reversed waveform starts with the known short negative tail instead of the original -1 peak");
                    require(last.r == 208 && last.g == 208 && last.b == 208,
                            "Reversed waveform places the known signed -1 peak at the last source column");
                } else if (phase == 6) {
                    // Unchanged JavaScript oracle: reversed kick, region [.25,.5],
                    // source pixel (3,42) = 0x5d422d. The fixture also paints
                    // it at 1:1 so smooth enlargement cannot interpolate the oracle.
                    const auto source = window->get_spectral_image().to_rgba8();
                    require(source.has_value() && source->width() == 4 && source->height() == 64,
                            "Native spectral fixture retains its four source columns and 64 frequency rows");
                    const auto expected = source->begin()[42 * 4 + 3];
                    require(expected.r == 93 && expected.g == 66 && expected.b == 45,
                            "Native source image matches the unchanged independent reversed-spectrum oracle");
                    const auto last = pixel(13, 262);
                    require(last.r == 93 && last.g == 66 && last.b == 45,
                            "Reversed spectral preview matches the independently calibrated reference source column");
                } else if (phase == 3) {
                    // Source column 2 is centered at .625. In [.5,1] it maps
                    // to .25 of the 613px plot, plus the fixture's 10px inset.
                    const auto silence = pixel(160, 205);
                    require(silence.r == 19 && silence.g == 15 && silence.b == 12,
                            "Spectral viewport shows the selected silent source column instead of the full source");
                    bool croppedRulerStartsAtHalf = false;
                    for (const auto& tick : slint::testing::ElementHandle::find_by_accessible_label(window, "0.5s"))
                        croppedRulerStartsAtHalf |= tick.absolute_position().x < 40;
                    if (!croppedRulerStartsAtHalf)
                        for (const auto& tick : slint::testing::ElementHandle::find_by_accessible_label(window, "0.5s"))
                            std::cerr << "0.5s ruler tick at " << tick.absolute_position().x << '\n';
                    require(croppedRulerStartsAtHalf,
                            "Cropped waveform ruler starts at its actual source time instead of zero");
                } else {
                    // Below the 18px ruler, y205 samples the low-frequency
                    // band; x85 is outside every display switch and overlay.
                    const auto low = pixel(85, 205);
                    require(phase == 1 ? low.r == 242 && low.g == 230 && low.b == 211
                                       : low.r == 0 && low.g == 255 && low.b == 0,
                            "Spectral low-frequency pixel displays the independently calibrated preview and reacts to palette changes");
                }
                if (++phase == 16) { timer.stop(); slint::quit_event_loop(); }
                else if (phase == 0) window->set_test_peaks(sparse);
                else if (phase == 1) activate("Display Spectral");
                else if (phase == 2) theme.set_paper_1(color(0x00ff00));
                else if (phase == 3) window->set_view_start(.5);
                else if (phase == 4) { window->set_view_start(0); activate("Display Wave"); }
                else if (phase == 5) window->set_reversed(true);
                else if (phase == 6) { window->set_region_start(.25); window->set_region_end(.5); activate("Display Spectral"); }
                else if (phase == 8) window->set_test_state("unsupported");
                else if (phase == 9) window->set_test_peaks(std::make_shared<slint::VectorModel<dandrum_ui::WaveBucket>>());
                else if (phase == 10) { window->set_test_state("missing"); window->set_test_peaks(sparse); }
                else if (phase == 11) {
                    window->set_test_state("loaded");window->set_plot_width(928);
                    std::vector<dandrum_ui::WaveBucket> variable;
                    for(int i=0;i<600;++i) variable.push_back(i%2?dandrum_ui::WaveBucket{-.8f,.6f}:dandrum_ui::WaveBucket{-.6f,.8f});
                    window->set_test_peaks(std::make_shared<slint::VectorModel<dandrum_ui::WaveBucket>>(std::move(variable)));
                }
                else if (phase == 12) { /* Keep the known overview bounds and source. */ }
                else if (phase == 13) window->set_main_zoom(2);
                else if (phase == 14) {window->set_main_zoom(1);window->set_test_state("loading");}
                else if (phase == 15) {
                    window->set_test_state("loaded");
                    window->set_test_slices(std::make_shared<slint::VectorModel<dandrum_ui::WaveSlice>>(
                        std::vector<dandrum_ui::WaveSlice>{{"slice-one","Slice one",0,1}}));
                }
                else { window->set_reversed(false); window->set_region_start(0); window->set_region_end(1); activate("Display Wave"); }
            } catch (const std::exception& error) { failure = error.what(); timer.stop(); slint::quit_event_loop(); }
        });
        window->run();
        if (!failure.empty()) throw std::runtime_error(failure);
        require(phase == 16, "Both display modes, reversal, viewport crop, live palette, source feedback and non-overlapping slice metadata are rendered");
        std::cout << "PASS: " << checks << " actual native waveform renderer checks\n";
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
