# Source-to-shell visual comparison

The [comparison board](evidence/final/comparisons.html) pairs unmodified supplied
renders and current shell captures. All 24 source states, all 24 action-reached
states and all eight pages at Min/Default/Expanded plus Compact are retained in
`evidence/final/{reference,actions,geometry,pages}`. Geometry JSON records exact
outer sizes, no internal horizontal scrolling and no text below 11px.

The reference reports nominal content dimensions plus its border (e.g. 1202×802).
The shell honors the requested exact outer 1200×800 (and other listed geometries).
The user's 11px minimum changes the received 8–10px labels. Menus/fields retain
accessible names while redundant table captions are hidden. Larger labels wrap
voice controls and Effects chains where required; vertical page scrolling makes
all controls reachable. Below 820px, the fixed editor remains readable and the
browser scrolls horizontally outside it. The supplied mobile preview instead
scales the native editor down; this is an intentional viewport-wrapper difference.

Header resource/voice counts, source/browser lists, operation details, inspector
positions, playheads and host tint come from actual accepted data/telemetry.
They differ from the reference's fixed 31 samples/6 voices and illustrative asset
lists. Knob hover/focus shows values and overlays; screenshots with keyboard focus
include the supplied focus outline. Filter modulation rings may show clipping
when summed route depths exceed its normalized range.

Envelope curves, handles and display values now share canonical parameters.
Time fields show 1ms/12ms rather than rounding small durations to 0.00s; Amp sustain
uses dB and Filter sustain uses percent. Source fixture fades are displayed from
actual duration (19 ms in, 386 ms out), correcting the source's contradictory 2 ms
caption. Detector/reduction are truthful read-only mock values (0 dB reduction).

The final read-only audit found clipped Effects controls, incorrect Lush/nested
candidate preview identity and envelope precision loss. All three are corrected
and protected by `shared-controls` browser tests and `model-bindings`/
`control-regressions` unit tests. Previously corrected findings include repeated
row labels, Lush in Sources, routing children density, chain details and shared
envelope geometry. No claim of pixel-exact 1px agreement is made across the
user-authorized typography/reflow and live-data differences described above.
