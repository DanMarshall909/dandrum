#!/usr/bin/env python3
"""Generate self-authored placeholder metallic samples for the 909-style
sampler voices (closed hat, open hat, crash, ride).

These are deliberately synthetic noise bursts authored from scratch so the
repository ships NO proprietary drum-machine samples (see the
`drum-voice-authoring` change, task 4.3). They exist only so the sampler-backed
909 voices load and render in tests; production users are expected to point the
sampler assets at their own licensed samples.

The generator is deterministic (fixed LCG seed) so regenerating produces
byte-identical WAVs.
"""
import math
import os
import struct
import wave

SAMPLE_RATE_HZ = 48_000
OUTPUT_DIR = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "examples",
    "assets",
    "drums",
    "909",
)

# name -> (duration_seconds, decay_time_constant_seconds, highpass_mix, seed)
# highpass_mix biases the noise toward brighter (metallic) content by mixing in
# the first difference of the noise stream.
VOICES = {
    "hat-closed": (0.08, 0.020, 0.9, 1),
    "hat-open": (0.45, 0.180, 0.9, 2),
    "crash": (1.60, 0.900, 0.8, 3),
    "ride": (1.20, 0.700, 0.6, 4),
}


def noise_stream(seed, count):
    """Deterministic white noise in [-1, 1] via a simple LCG."""
    state = seed & 0xFFFFFFFF
    for _ in range(count):
        state = (1_664_525 * state + 1_013_904_223) & 0xFFFFFFFF
        yield (state / 0xFFFFFFFF) * 2.0 - 1.0


def render(duration_s, decay_s, highpass_mix, seed):
    total = int(SAMPLE_RATE_HZ * duration_s)
    raw = list(noise_stream(seed, total))
    samples = []
    previous = 0.0
    for index, value in enumerate(raw):
        # Brighten toward metallic content by mixing the high-frequency
        # first difference with the raw noise.
        bright = (1.0 - highpass_mix) * value + highpass_mix * (value - previous)
        previous = value
        envelope = math.exp(-index / (SAMPLE_RATE_HZ * decay_s))
        samples.append(bright * envelope * 0.6)
    return samples


def write_wav(path, samples):
    with wave.open(path, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(SAMPLE_RATE_HZ)
        frames = bytearray()
        for sample in samples:
            clamped = max(-1.0, min(1.0, sample))
            frames += struct.pack("<h", int(clamped * 32767))
        handle.writeframes(bytes(frames))


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    for name, params in VOICES.items():
        samples = render(*params)
        path = os.path.join(OUTPUT_DIR, f"{name}.wav")
        write_wav(path, samples)
        print(f"wrote {path} ({len(samples)} frames)")


if __name__ == "__main__":
    main()
