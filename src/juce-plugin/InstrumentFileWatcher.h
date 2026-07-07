#pragma once

#include <functional>

#include <juce_core/juce_core.h>
#include <juce_events/juce_events.h>

/// Watches a single instrument YAML file for external edits off the audio
/// thread and, once a change has stabilised, invokes a reload callback.
///
/// The change signal is a cheap (modification-time, size) pair, falling back to
/// a content hash when the filesystem reports no usable modification time. A
/// detected change must repeat unchanged across `kStableChecksRequired`
/// consecutive polls before it is treated as stable, so a partially-written
/// file cannot trigger a premature reload.
///
/// All mutable state is guarded so the internal polling timer (message thread)
/// and any thread that re-points the watcher after a reload cannot race. The
/// reload callback is always invoked outside the lock so it may safely re-point
/// the watcher.
class InstrumentFileWatcher final : private juce::Timer
{
public:
    using ReloadCallback = std::function<void (const juce::File&)>;

    /// Low-frequency polling interval. File watching is a background
    /// convenience, not a realtime signal, so this stays well clear of the
    /// audio thread's cadence.
    static constexpr int kPollIntervalMs = 400;

    /// Number of consecutive polls a change signal must remain unchanged before
    /// it is treated as stable and reloaded. Debounces partially-written files.
    static constexpr int kStableChecksRequired = 2;

    InstrumentFileWatcher();
    ~InstrumentFileWatcher() override;

    /// Installs the callback invoked when a stable external change is detected.
    void onReload (ReloadCallback callbackToUse);

    /// Points the watcher at a file and primes its baseline to the file's
    /// current state, so only edits made after this call trigger a reload.
    void watchFile (const juce::File& fileToWatch);

    /// Stops watching and clears the current file.
    void stopWatching();

    /// Enables/disables watching. While disabled, external edits are ignored
    /// and never trigger a reload until watching is re-enabled.
    void setEnabled (bool shouldBeEnabled);
    bool isEnabled() const noexcept;

    const juce::File& watchedFile() const noexcept;

    /// Polls the watched file once for a stable external change and fires the
    /// reload callback if one is detected. Called by the internal timer in the
    /// plugin; exposed publicly so tests can drive polling deterministically
    /// without a running message loop.
    void poll();

private:
    struct ChangeSignal
    {
        juce::int64 modificationTimeMs = 0;
        // -1 means "no readable file" so a missing file never compares equal to
        // any real content and never registers as a stable change.
        juce::int64 sizeBytes = -1;
        juce::int64 contentHash = 0;

        bool operator== (const ChangeSignal& other) const noexcept;
        bool operator!= (const ChangeSignal& other) const noexcept { return ! (*this == other); }
    };

    static ChangeSignal readSignal (const juce::File& file);
    void updateTimerState();
    void timerCallback() override;

    juce::CriticalSection lock;
    ReloadCallback reloadCallback;
    juce::File file;
    bool enabled = true;
    ChangeSignal appliedSignal;
    ChangeSignal pendingSignal;
    int pendingStableCount = 0;
    bool hasPending = false;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (InstrumentFileWatcher)
};
