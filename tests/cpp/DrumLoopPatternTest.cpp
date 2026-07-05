#include <cassert>

#include "../../src/juce-wrapper/DrumLoop.h"

int main()
{
    const auto hits = dandrum::makeSimpleDrumLoop();

    assert (hits.size() == 12);
    assert (hits[0].note == dandrum::drumLoopKickNote);
    assert (hits[3].note == dandrum::drumLoopSnareNote);
    assert (hits[1].note == dandrum::drumLoopHatNote);
    assert (hits[0].startOffset.count() == 0);
    assert (hits[3].startOffset.count() == 250);
    assert (hits[8].startOffset.count() == 625);

    return 0;
}
