#pragma once
#include "ui/slint/host/SpectralPreview.h"

// Compatibility adapter for existing sampler bindings and pixel-oracle tests.
namespace dandrum::advanced_sampler {
using dandrum::slint_ui::spectral_rows;
using dandrum::slint_ui::spectral_max_columns;
using dandrum::slint_ui::SpectralPixels;
using dandrum::slint_ui::SpectralPalette;
using dandrum::slint_ui::spectral_pixels;
using dandrum::slint_ui::WaveDisplayCache;
namespace spectral_detail = dandrum::slint_ui::spectral_detail;
}
