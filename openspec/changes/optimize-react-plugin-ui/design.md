## Context

Linux JUCE/WebKit measurements show TB-303 at 71/138 ms median/p95 command latency and 105% of one core idle; suppressing its parent transform reduces both substantially. Both editors publish and reconcile unchanged state and refresh synchronously in the gesture command path. WebView baseline memory remains substantial.

## Goals / Non-Goals

Goals: preserve native gesture ordering and generation safety, bound asynchronous snapshot work, avoid unchanged visual updates, fit TB-303 at native editor widths with readable labels, and verify improvements using the same production assets/native engine.

Non-goals: replace WebKit, change DSP, finish unrelated renderer UI tasks, or promise platform-independent resource figures.

## Decisions

Use normal CSS layout with compact grid breakpoints rather than a whole-panel transform. Keep controls at their native interactive sizes and labels at least 11px. Verify native computed layout at 760, 820, 1180 and 1500px.

Keep the centered frame's height synchronized with the unscaled panel using a ResizeObserver, writing only when geometry changes. Native diagnostic runs show that allowing the centered ancestor to size automatically retains expensive shadow repainting during control changes; a stable frame removes that delay while preserving the shadows and responsive dimensions.

Extract one shared parameter controller and React hook. Admit commands as soon as the native reply arrives. Refresh only after writes, allowing one read in flight with one replacement refresh. Preserve rejection reporting, stale-state filtering and generation changes. Skip equal state identities while allowing host automation with an equal command sequence. Gesture work captures its originating interaction before serializer waits and retains it through queued and coalesced operations. The knob adapter consumes that ownership metadata before invoking the native bridge.

Native publication remembers the last visible snapshot per bridge. Compare generation, sequence and all parameter fields before serializing another publication; reset the remembered state when hidden or reloaded. Retain exactly one unacknowledged publication and editor-owned tickets.

Normalize meter packets then compare the current rendered display. Transport acknowledgement still runs on every packet. Stop prepared timers only on completed/failure/cancellation/staleness; paged spectral readiness is terminal only after assembly.

## Risks / Trade-offs

- Asynchronous reads can finish across writes or reloads → generation/sequence guards, pending-write filtering and a replacement read catch up safely.
- Equal command sequences can still contain host automation → compare actual parameter values as well as sequence.
- Clip acknowledgements locally change display → compare against current React display, not a transport packet cache.
- CSS changes can overflow narrow editors → runtime layout assertions and screenshots at supported widths.
- WebView memory dominates editor use → report measured memory honestly; this slice primarily reduces CPU and latency.

## Migration Plan

Rebuild the existing offline bundles and native embedded targets. No document migration is needed. A revert restores the previous UI paths.
