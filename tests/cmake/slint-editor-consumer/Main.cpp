#include "Consumer.h"
#include "host/AppearanceBinding.h"
#include "host/AppearanceSettings.h"
#include "host/DesignTokens.h"
#include "host/MaterialImage.h"
#include "host/ModelProjection.h"
#include "host/SearchFilter.h"
#include "host/SpectralPreview.h"
#include "host/ThemeBinding.h"
#include "host/ValueCodec.h"
#include "host/VelocityBlend.h"
#include "platform/FileDropProtocol.h"

#include <iostream>
#include <type_traits>

// Compilation covers the public UI types and all independently supplied host
// helpers. These value checks run without opening a window or loading an engine.
int main()
{
    using namespace dandrum::slint_ui;
    static_assert(std::is_class_v<independent_ui::IndependentEditor>);
    static_assert(tokens::editor_musical_notes_per_octave == 12);
    static_assert(tokens::editor_musical_midi_maximum == 127);
    static_assert(file_drop::maximum_files == 256);
    const ThemeSettings settings;
    if (settings.accent != 0xe8c47a || velocity_blend_gain(0.5, 1, 127, 0, "none") != 1.) {
        std::cerr << "Independent consumer default value contract failed\n";
        return 1;
    }
    std::cout << "PASS: independent umbrella and C++ host consumer compiled and linked\n";
}
