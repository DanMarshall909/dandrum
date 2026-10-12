#pragma once
#include <slint.h>
#include "../../slint/host/SearchFilter.h"
#include <memory>

namespace dandrum::sampler {
template<class PatchFilterGlobal>
void bind_patch_search(const PatchFilterGlobal& filter) { dandrum::slint_ui::bind_search_filter(filter); }

// The standalone and native app tests share one canonical performance bridge.
template<class PerformanceGlobal, class SessionBinding>
void bind_performance_sources(const PerformanceGlobal& performance, const std::shared_ptr<SessionBinding>& binding)
{
    performance.set_base_note(static_cast<int>(binding->model()->number("octave-base")));
    performance.set_pc_enabled(binding->model()->number("pc-keys") != 0);
    const std::weak_ptr<SessionBinding> weak(binding);
    performance.on_release_notes([weak] {
        if (const auto live=weak.lock()) { live->model()->releaseNotes(); live->refresh(); }
    });
    performance.on_octave_changed([weak](int note) {
        if (const auto live=weak.lock()) {
            live->model()->command("pc.octave", "", "", note-live->model()->number("octave-base"));
            live->refresh();
        }
    });
    performance.on_pc_changed([weak](bool enabled) {
        if (const auto live=weak.lock()) { live->model()->command("pc.enabled", "", "", enabled?1:0); live->refresh(); }
    });
}
}
