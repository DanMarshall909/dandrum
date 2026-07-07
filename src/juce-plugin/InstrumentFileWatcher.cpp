#include "InstrumentFileWatcher.h"

bool InstrumentFileWatcher::ChangeSignal::operator== (const ChangeSignal& other) const noexcept
{
    return modificationTimeMs == other.modificationTimeMs
           && sizeBytes == other.sizeBytes
           && contentHash == other.contentHash;
}

InstrumentFileWatcher::InstrumentFileWatcher() = default;

InstrumentFileWatcher::~InstrumentFileWatcher()
{
    stopTimer();
}

void InstrumentFileWatcher::onReload (ReloadCallback callbackToUse)
{
    const juce::ScopedLock scopedLock (lock);
    reloadCallback = std::move (callbackToUse);
}

InstrumentFileWatcher::ChangeSignal InstrumentFileWatcher::readSignal (const juce::File& file)
{
    ChangeSignal signal;
    if (! file.existsAsFile())
        return signal;

    signal.modificationTimeMs = file.getLastModificationTime().toMilliseconds();
    signal.sizeBytes = file.getSize();

    // Fall back to a content hash only when the filesystem reports no usable
    // modification time; the mtime/size pair is enough on every platform that
    // does. Hashing every poll would defeat the point of a cheap change signal.
    if (signal.modificationTimeMs == 0)
        signal.contentHash = file.loadFileAsString().hashCode64();

    return signal;
}

void InstrumentFileWatcher::watchFile (const juce::File& fileToWatch)
{
    {
        const juce::ScopedLock scopedLock (lock);
        file = fileToWatch;
        appliedSignal = readSignal (file);
        pendingSignal = {};
        pendingStableCount = 0;
        hasPending = false;
    }

    updateTimerState();
}

void InstrumentFileWatcher::stopWatching()
{
    {
        const juce::ScopedLock scopedLock (lock);
        file = juce::File();
        pendingSignal = {};
        pendingStableCount = 0;
        hasPending = false;
    }

    updateTimerState();
}

void InstrumentFileWatcher::setEnabled (bool shouldBeEnabled)
{
    {
        const juce::ScopedLock scopedLock (lock);
        enabled = shouldBeEnabled;
        if (! enabled)
        {
            pendingSignal = {};
            pendingStableCount = 0;
            hasPending = false;
        }
    }

    updateTimerState();
}

bool InstrumentFileWatcher::isEnabled() const noexcept
{
    const juce::ScopedLock scopedLock (lock);
    return enabled;
}

const juce::File& InstrumentFileWatcher::watchedFile() const noexcept
{
    const juce::ScopedLock scopedLock (lock);
    return file;
}

void InstrumentFileWatcher::updateTimerState()
{
    const auto shouldRun = [this]
    {
        const juce::ScopedLock scopedLock (lock);
        return enabled && file != juce::File();
    }();

    if (shouldRun)
        startTimer (kPollIntervalMs);
    else
        stopTimer();
}

void InstrumentFileWatcher::poll()
{
    ReloadCallback callbackToFire;
    juce::File fileToReload;

    {
        const juce::ScopedLock scopedLock (lock);

        if (! enabled || file == juce::File())
            return;

        const auto current = readSignal (file);

        // A missing or not-yet-readable file (e.g. an editor mid-save that has
        // truncated/renamed it) is not a stable change: keep the running
        // instrument and wait for the file to become readable again.
        if (current.sizeBytes < 0)
        {
            pendingStableCount = 0;
            hasPending = false;
            return;
        }

        if (current == appliedSignal)
        {
            pendingStableCount = 0;
            hasPending = false;
            return;
        }

        if (hasPending && current == pendingSignal)
        {
            ++pendingStableCount;
        }
        else
        {
            pendingSignal = current;
            pendingStableCount = 1;
            hasPending = true;
        }

        if (pendingStableCount < kStableChecksRequired)
            return;

        appliedSignal = current;
        pendingStableCount = 0;
        hasPending = false;
        callbackToFire = reloadCallback;
        fileToReload = file;
    }

    // Fired outside the lock so the callback may re-point the watcher.
    if (callbackToFire)
        callbackToFire (fileToReload);
}

void InstrumentFileWatcher::timerCallback()
{
    poll();
}
