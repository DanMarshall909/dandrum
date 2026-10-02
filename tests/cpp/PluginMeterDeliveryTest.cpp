#include "InstrumentUiMeterDelivery.h"

#include <iostream>

namespace
{
using Delivery = InstrumentUiMeterDelivery;

bool check (bool condition, const char* message)
{
    if (! condition)
        std::cerr << message << '\n';
    return condition;
}

Delivery::Packet packet (std::uint32_t generation, std::uint64_t sample)
{
    Delivery::Packet result;
    result.meter.generation = generation;
    result.meter.endSample = sample;
    result.clip.generation = generation;
    result.clip.valid = true;
    return result;
}
}

int main()
{
    static_assert (sizeof (Delivery) <= 4096);
    Delivery delivery;
    if (! check (delivery.subscribe (11, 7), "first meter session was rejected")
        || ! check (delivery.subscribe (22, 7), "second meter session was rejected")
        || ! check (delivery.visibleCount() == 2, "visible subscriber count is wrong")
        || ! check (! delivery.setVisible (999, false), "unknown session visibility changed")
        || ! check (delivery.setVisible (11, true), "visible session no-op was rejected")
        || ! check (! delivery.take (999).has_value(), "unknown session received data"))
        return 1;

    auto invalid = packet (7, 32);
    invalid.clip.valid = false;
    delivery.publish (invalid);
    invalid.clip.valid = true;
    invalid.clip.generation = 8;
    delivery.publish (invalid);
    if (! check (! delivery.take (11).has_value(),
                 "invalid or mismatched clip state was published"))
        return 1;
    delivery.publish (packet (7, 64));
    const auto first = delivery.take (11);
    if (! check (first.has_value() && first->meter.endSample == 64,
                 "first packet was not delivered")
        || ! check (! delivery.take (11).has_value(),
                    "unacknowledged session accepted a second packet"))
        return 1;

    for (std::uint64_t sample = 128; sample <= 64000; sample += 64)
        delivery.publish (packet (7, sample));
    if (! check (delivery.pendingPayloads() <= 4,
                 "stalled sessions retained unbounded meter payloads")
        || ! check (! delivery.acknowledge (11, 7, first->sequence + 1),
                    "incorrect acknowledgement advanced the meter session")
        || ! check (! delivery.take (11).has_value(),
                    "stalled session replayed meter history")
        || ! check (delivery.acknowledge (11, 7, first->sequence),
                    "current packet acknowledgement failed"))
        return 1;
    if (! check (! delivery.acknowledge (11, 7, first->sequence),
                 "duplicate acknowledgement advanced the session"))
        return 1;

    const auto resumed = delivery.take (11);
    if (! check (resumed.has_value() && resumed->meter.endSample == 64000
                 && resumed->sequence > first->sequence + 1,
                 "resumed subscriber did not receive coalesced current data"))
        return 1;

    delivery.setVisible (11, false);
    if (! check (delivery.visibleCount() == 1 && ! delivery.take (11).has_value(),
                 "hidden meter session retained publication")
        || ! check (! delivery.acknowledge (11, 7, resumed->sequence),
                    "hidden meter session accepted an acknowledgement"))
        return 1;
    delivery.setVisible (22, false);
    if (! check (delivery.visibleCount() == 0, "last hidden session left capture enabled"))
        return 1;
    delivery.publish (packet (7, 64064));
    delivery.setVisible (11, true);
    if (! check (delivery.visibleCount() == 1 && ! delivery.take (11).has_value(),
                 "reopened subscriber replayed a hidden packet"))
        return 1;
    delivery.publish (packet (7, 64128));
    const auto visibleAgain = delivery.take (11);
    if (! check (visibleAgain.has_value() && visibleAgain->meter.endSample == 64128,
                 "visible subscriber did not receive fresh data"))
        return 1;

    if (! check (! delivery.subscribe (11, 8), "duplicate session ID was admitted")
        || ! check (delivery.unsubscribe (11), "session close failed")
        || ! check (! delivery.acknowledge (11, 7, visibleAgain->sequence),
                    "closed session accepted an acknowledgement")
        || ! check (delivery.subscribe (33, 8), "reopened session was rejected"))
        return 1;
    delivery.publish (packet (7, 64200));
    if (! check (! delivery.take (33).has_value(),
                 "new generation received stale meter data"))
        return 1;
    delivery.publish (packet (8, 128));
    const auto reopened = delivery.take (33);
    if (! check (reopened.has_value() && reopened->meter.generation == 8
                 && reopened->meter.endSample == 128,
                 "reopened session missed its current generation")
        || ! check (! delivery.acknowledge (33, 7, reopened->sequence),
                    "old generation acknowledged a new packet")
        || ! check (delivery.acknowledge (33, 8, reopened->sequence),
                    "new generation acknowledgement failed"))
        return 1;

    if (! check (delivery.subscribe (44, 8), "third session was rejected")
        || ! check (delivery.subscribe (55, 8), "fourth session was rejected")
        || ! check (! delivery.subscribe (66, 8), "session table grew past its limit")
        || ! check (delivery.unsubscribe (22), "hidden session close failed")
        || ! check (! delivery.unsubscribe (22), "unknown session close succeeded")
        || ! check (delivery.subscribe (66, 8), "freed session slot was not reused"))
        return 1;
    if (! check (delivery.unsubscribe (66), "session slot could not be retired")
        || ! check (delivery.unsubscribe (55), "second session slot could not be retired")
        || ! check (delivery.subscribe (77, 9), "new-generation session was rejected"))
        return 1;
    delivery.retireOtherGenerations (9);
    if (! check (delivery.visibleCount() == 1,
                 "reload left old-generation capture subscriptions active")
        || ! check (! delivery.acknowledge (33, 8, reopened->sequence),
                    "retired generation accepted a late acknowledgement")
        || ! check (delivery.subscribe (33, 9),
                    "retired session ID could not be reused after reload"))
        return 1;
    return 0;
}
