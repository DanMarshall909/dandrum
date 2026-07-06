#pragma once

#include <atomic>
#include <chrono>
#include <thread>
#include <vector>

namespace dandrum
{
struct DrumLoopHit
{
    int note = 0;
    int velocity = 0;
    std::chrono::milliseconds startOffset { 0 };
    std::chrono::milliseconds duration { 0 };
};

inline constexpr int drumLoopKickNote = 36;
inline constexpr int drumLoopSnareNote = 38;
inline constexpr int drumLoopHatNote = 42;
inline constexpr int drumLoopVelocity = 110;
inline constexpr std::chrono::milliseconds drumLoopHatDuration { 70 };

inline std::vector<DrumLoopHit> makeSimpleDrumLoop()
{
    return {
        { drumLoopKickNote, drumLoopVelocity, std::chrono::milliseconds { 0 }, std::chrono::milliseconds { 120 } },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 0 }, drumLoopHatDuration },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 125 }, drumLoopHatDuration },
        { drumLoopSnareNote, drumLoopVelocity, std::chrono::milliseconds { 250 }, std::chrono::milliseconds { 120 } },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 250 }, drumLoopHatDuration },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 375 }, drumLoopHatDuration },
        { drumLoopKickNote, drumLoopVelocity, std::chrono::milliseconds { 500 }, std::chrono::milliseconds { 120 } },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 500 }, drumLoopHatDuration },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 625 }, drumLoopHatDuration },
        { drumLoopSnareNote, drumLoopVelocity, std::chrono::milliseconds { 750 }, std::chrono::milliseconds { 120 } },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 750 }, drumLoopHatDuration },
        { drumLoopHatNote, drumLoopVelocity - 20, std::chrono::milliseconds { 875 }, drumLoopHatDuration },
    };
}

template <typename NoteOn, typename NoteOff>
void playSimpleDrumLoopOnce (const std::atomic<bool>& shouldQuit, NoteOn&& noteOn, NoteOff&& noteOff)
{
    const auto hits = makeSimpleDrumLoop();
    auto loopStart = std::chrono::steady_clock::now();

    for (const auto& hit : hits)
    {
        if (shouldQuit.load())
            return;

        const auto hitStart = loopStart + hit.startOffset;
        std::this_thread::sleep_until (hitStart);
        noteOn (hit.note, hit.velocity);

        const auto hitEnd = hitStart + hit.duration;
        std::this_thread::sleep_until (hitEnd);
        noteOff (hit.note);
    }
}
} // namespace dandrum
