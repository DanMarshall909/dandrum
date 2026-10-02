# React knob integration

This records Web and native progress on OpenSpec task 3.4 in
[add-renderer-independent-plugin-ui](../../openspec/changes/add-renderer-independent-plugin-ui/tasks.md).
The sampler and TB-303 use one adaptation of the preserved
[Knob reference](reference/components/controls/Knob.jsx). Task 3.4 now has paired
host and signed-audio evidence through both actual renderers.

## Current behavior

- The reference's pointer-free cap, 270-degree value arc, thin resting track,
  whole-cell cream focus, hover/focus popup and 36/48/64 logical-pixel sizes are
  shared by both React apps. Only the active value arc thickens.
- Commands use normalized values and the existing shared host service. Vertical
  drag uses 200 pixels per full range, or 800 with Shift. Arrows use 5%/1%, wheel
  uses 2%/1%, Home/End use the limits, and Delete/Backspace/double-click use the
  loaded instrument default. A drag retains one host gesture; discrete writes
  use the existing bounded/coalesced controller.
- The popup shows actual values from the admitted prepared range. Enter or a
  click opens typed entry; Enter/blur commits a complete in-range decimal,
  Escape cancels, and malformed entry restores the authoritative value.
  Closing an unchanged rounded readout never rewrites the precise host value.
  Blur permits focus to move to the next control.
- Live values, ranges and defaults must share one instrument generation and
  public ID. Controls are unavailable until matching metadata arrives. One
  read-only request is in flight; superseded replies cannot restore old ranges.
  Editor closure ignores late replies and errors.
- A non-passive local wheel listener prevents page scrolling during value
  changes. Popup interaction cannot accidentally drag, wheel or reset the knob.
- Modulation assignment and automation provenance require engine/host data
  beyond the current document. Their illustrative rings and callbacks remain
  unavailable. Structural runtime work remains in section 9 of the task list.

The sampler's old sliders and normalized fields, TB-303's duplicate gesture
implementation and pointer drawing, and unused formatting/refresh helpers are
removed. The source reference, Rust engine, FFI and host parameter identities
are preserved.

## Verification

The two original production editor factories run in their actual Linux WebKit
browsers through `sampler-web-runtime` and `tb303-web-runtime`. They observe:

- eight embedded licensed font faces, supplied icons and the actual 23/7 host
  surfaces at 1200×800 and 820×560;
- host updates of 0.21 → 0.37 → 0.63 through the normal timer;
- actual popup values of 1.77875 for sampler pitch (range 0.125–8) and 0.2048 for
  TB-303 cutoff (range 0.02–0.9) at normalized 0.21;
- keyboard, loaded reset, wheel, typed actual entry, invalid/cancelled/unchanged
  entry and blur, with eleven balanced host gestures on the original slot and
  normalized host value 0.75 after those eleven edits;
- wheel cancellation and focus leaving typed entry;
- one additional sampler hold/release gesture that receives a normal host
  automation update while held and displays 0.84 on closure. This is the owning
  real-host regression for the shared component.

The hold uses a synthetic pointer down/up after an actual native mouse move;
actual mouse-button input and pointer capture are separately checked in the
CDP component lane. The TB-303 boundary run verifies its distinct command/range
bindings through eleven discrete gestures; a TB-303 native pointer probe failed
to receive the mouse move and is not claimed as overlap evidence.

The observer waits for real processor-listener gesture ends between actions;
matching a value alone cannot prove an asynchronous gesture has ended. There is
no extra browser, replaced native function or manual host-state publication.
The observer exposes listener counts solely to coordinate the test.

Both copied complete executables passed outside hidden source checkouts, with
fresh application data and no network route. Full/compact screenshots were
inspected. They show the new knob within the existing layouts; the sampler's
full layout, labels and remaining displays still need task 7.5's adaptation.
These JUCE test applications do not prove VST3 loading or transport continuity
inside a DAW.

The normal CTest pipeline includes focused value/input and document-request
Node suites. Deterministic Promise barriers cover pending/superseded metadata,
retry and closure. Their V8 reports execute all ranges in the two new policy
modules (12 and 8 functions). A separate real React DOM component run exercises
actual pointer capture, both drag sensitivities, popup isolation, rejection and
reload with a recording host. A deterministic end/write barrier also asserts
that release reconciles suppressed host changes without overwriting a newer
local value before its host request completes. All 42 knob functions execute; no executable
source line is entirely unexecuted. Three defensive null-to-zero reconciliation
fragments remain unexecuted, so complete branch coverage is not claimed.
The recording host proves only the command subset calibrated by the real
WebKit tests; it does not model audio or engine ownership.

Seven copied-source faults were rejected by named assertions: wrong loaded
reset, actual-range offset loss, stale metadata admission, obsolete document
publication and a normalized popup readout, omitted release reconciliation and overwriting a newer pending
local edit. Mutation runs exclude end-to-end
checks. Browser source attribution verifies unchanged function bodies after
binding ES module imports/exports for the component fixture.

Both frontend unit/typecheck/build commands pass. Full Web CMake/CTest passes
42/42 without skips; native-only build/CTest passes 18/18. Strict OpenSpec and
baseline spec coverage pass; delta specs remain unsynced. Detailed logs, RED
runs, source hashes, fault results, browser coverage and original-editor captures
are retained at `/tmp/dandrum-knob-react-evidence` for completion review.

## Native primary control

The native editor's primary public control now uses the reference's 64-pixel
pointer-free cap, 270-degree arc, generated colors, whole-cell focus rim and
actual-value popup. Caption and value text use embedded Barlow Semi Condensed
SemiBold and JetBrains Mono Medium; both OFL notices are embedded alongside them.
The rest of the native layout remains in task 7.5.

The control owns copies of its prepared range, default and generation. Reload
cancels old drafts and captured drags. Missing, zero-width and non-finite actual
ranges disable editing. Typed entry, keyboard, wheel and loaded reset use the
shared host command service, with the same steps as React. A continuous drag
uses one host gesture at 200 pixels per full range, or 800 with Shift. Release,
focus loss and editor closure close it once. Release reconciles host automation
that arrived while the local drag was displayed.

Enter, Escape and blur finish their current entry synchronously. A delayed JUCE
text message cannot finish a subsequently reopened draft. Callback copies and
`Component::SafePointer` checks stop continuations if a synchronous host listener
closes the editor. The shared service remembers closure during a pending begin
and retires gesture ownership before notifying the host of an end.

The existing 30 Hz observer expires the 250 ms pointer handoff grace and 600 ms
nudge display without a new timer or worker. Only the active value arc thickens.
Popup clicks and entry cannot become drag, wheel or reset commands.

`native-editor-smoke` drives the actual native virtual input handlers and JUCE
message queue against a real processor. It checks stable host slots, balanced
gestures, actual text/typefaces, cap/arc pixels and every signed stereo sample
of the prepared knob fixture. It includes malformed/unchanged/rounded entry,
host updates during drafts and drags, reload, closure and timed popup states.
Six real host-listener cases close the editor during begin, change, end, typed
completion, focus loss and reload; they assert every event's original host slot,
balanced gestures, literal signed PCM and replacement-editor recovery. A focused
AddressSanitizer run caught the original use-after-free and passes after repair;
only the owned native editor and test are instrumented, with leak checking off.
The fast `cxx-plugin-ui-command` test independently checks reentrant session
closure. Native CTest passes 19/19. Focused copied-source faults reject eight
native control defects and three service ownership defects by named assertions.
Both native and Web standalone/VST3 builds pass; the current Web CTest run
passes 43/43, including the two original WebKit editor runtime checks.

Merged coverage executes 332 of 336 new font/knob source lines; four defensive
returns after JUCE focus/visibility operations remain unexecuted. The isolated
service test executes all 48 measured begin/end/close lines; its parameter-write
method is not instantiated in that isolated run. Full module and exhaustive
branch coverage are not claimed; no production module is extracted.

The maintained test's events are synthetic. A separate X11 XTest probe drives
physical button/motion and Shift input through the original native editor,
including a captured drag beyond the cell and host automation while held. It
checks the same stable host slot and every signed PCM sample, and captures the
focused popup at 820×560 and 1200×800. These JUCE applications do not prove DAW
transport behavior. Focused coverage and verification evidence is retained at
`/tmp/dandrum-knob-native-evidence`.

## Paired renderer schedule

[PluginKnobParityTest.cpp](../../tests/cpp/PluginKnobParityTest.cpp) compiles into
the native-only and Web configurations as `native-knob-parity` and
`web-knob-parity`. Both start with the original sampler factory and load the
same signed gain fixture with a distinguishable prepared reset value of 0.25.
They use the actual native handlers/TextEditor and shipped React DOM handlers;
the Web observer does not replace browser options, native functions or the
normal host observer.

One literal schedule covers a two-update drag, typed zero, keyboard, fine wheel,
loaded reset, cancellation and host automation. Each action waits for its host
notifications and displayed state, then renders exactly 64 frames at 48 kHz.
The Web driver waits for C++'s acknowledgement of that render before advancing.
Both assert the same ordered five begin/end gestures and seven value changes,
the original slot-zero parameter object and unchanged instrument generation.
All 704 stereo frames (1,408 channel samples) match independent f32 expectations exactly;
display precision is checked separately. Signed examples include -0.5, +0.5,
zero and the f32 results near +0.1, +0.08 and +0.68.

For each actual renderer, a copied fixture whose source input changes from 1 to
0.5 fails the signed-PCM assertion at channel zero/frame 128. Restoring it passes.
This calibrates the audio oracle, without claiming production mutation coverage.
Full builds pass and current CTest passes native 20/20 and Web 44/44 without
skips. Exact traces, fixture faults and commands are retained at
`/tmp/dandrum-knob-parity-evidence`. No production code changes in this proof
slice; the earlier implementation coverage gaps remain as recorded above.
These runtime actions are synthetic and do not prove DAW loading or transport.

## Remaining scope

Prepared waveform/spectral displays, KeyMap/LayerStack/OutputBusses adaptation,
full and compact design fidelity, callback/stress evidence and automatic
structural rebuilding retain their own unchecked tasks. This slice makes no
completion claim for those features.
