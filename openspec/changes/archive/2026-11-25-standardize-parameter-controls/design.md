## Context

Trigger currently adapts a supplied rotary knob and numeric field while formatting values through several label-specific helpers. The engine and host use stable normalized or native parameter values, but the UI independently decides how to display, parse and step percentages, time, pitch, gain, pan and processor values. The catalog also presents its generic knob as a fixed percentage control. This makes behavior inconsistent and encourages isolated event-handler fixes.

The delivered knob already has pointer, wheel, keyboard, typed-entry, reset and host-gesture behavior. This change extends that foundation without changing engine parameter identities or DSP transfer functions.

## Goals / Non-Goals

**Goals:**

- Give every continuous or stepped parameter one declarative presentation and interaction descriptor.
- Make rotary knobs and numeric fields share formatting, parsing, range, step, transfer, scale and accessibility semantics.
- Support direct entry in displayed units, exact range fractions, unit-interval and percentage forms, bipolar controls and pan.
- Preserve rational intent across later drags, authoritative numeric precision, one-operation Undo and balanced host automation gestures.
- Make all variants reviewable in the isolated component catalog.

**Non-Goals:**

- Change normalized engine storage, stable public parameter IDs, DSP behavior or host automation slots.
- Add a general expression language or arithmetic beyond a single signed integer fraction.
- Implement modulation routing, tempo transport or new parameter types that do not already have an owning product model.
- Redesign unrelated Trigger layouts, sample-browser behavior or prepared structural controls.

## Decisions

### One descriptor owns presentation and interaction

Each bound parameter will expose a descriptor containing its native minimum, maximum and default; normalized-to-native and native-to-normalized transfer; display variant and unit; ordinary, fine and coarse steps; parser and formatter; scale marks; discrete choices where applicable; and accessible value formatter. Knobs, fields and catalog stories consume the same descriptor.

This replaces label-based conditionals as the authority. Existing helpers can initially adapt to the descriptor so migration remains incremental. Keeping independent knob and field parsers was rejected because it would preserve the inconsistency this change is intended to remove.

### Normalized position is the range-fraction coordinate

A fraction `n/d` represents the normalized position `n/d` across a unipolar range. On a symmetric bipolar descriptor, its sign selects direction from the centre and its magnitude is the proportion of that side, so `1/3` and `-1/3` are symmetric. Transfer curves are applied after this normalized interpretation; one-third on a logarithmic frequency control therefore means one-third of the control travel rather than one-third of the hertz span.

Physical-unit entry continues to address native values. A percentage descriptor can display either `54%` over `0–100%` or `.54` over `0–1`. Decimals follow the displayed unit, while a fraction always denotes a range proportion. Unit-interval values omit the leading zero between -1 and 1, except that zero remains `0`.

### Rational edit state preserves the entered denominator

Successful fraction entry records a transient rational edit state `{numerator, denominator}` alongside the authoritative numeric value. Subsequent upward or downward dragging changes the numerator by one per value step and retains the entered denominator, including unreduced forms such as `2/6`. Shift changes the pointer distance needed for a step without changing the mathematical `1/d` increment. Coarse stepping may move by multiple numerators.

The passive value may use the descriptor's rounded decimal display, but reopening the unchanged editor restores the entered fraction. A decimal or physical-unit entry, reset, incompatible host update, descriptor generation change or new binding clears rational mode. This avoids pretending that the engine or host stores arbitrary-precision rational numbers while still retaining the musician's editing intent.

### Display variants remain explicit

Descriptors distinguish percentage, unit interval, bipolar percentage, bipolar unit interval, pan, physical-unit, tempo-synced time and stepped variants. Bipolar value arcs originate at the centre and use symmetric marks with a prominent zero. Pan stores its ordinary signed position but displays `L 54%`, `C` or `R 54%`; the DSP pan law is outside the presentation contract.

Frequency and appropriate time controls can use nonlinear transfer curves. Pitch can provide semitone steps and stronger zero or octave marks. Integer, enumerated and other discrete descriptors snap before publication and never display unsupported intermediate decimals.

### Musical note values use explicit syntax

Bare fractions retain the universal range-proportion meaning. Tempo-synced time uses explicit note notation such as `1/8 note`, `1/8D` and `1/8T`. Tempo sync and note retrigger remain separate parameter concerns. This avoids assigning two meanings to `1/8` based on hidden context.

### Editing and dragging have separate hit targets

Clicking the visible value opens its text editor. Pointer-down on the dial begins value dragging and cannot be intercepted by the popup. Enter commits valid text, Escape cancels, and blur commits only valid text. Invalid text stays open with an inline error. A valid out-of-range value clamps on commit and the resulting authoritative value is shown. Double-clicking the dial restores the declared default rather than opening entry.

Arrow keys use the ordinary displayed-unit step, Shift uses the fine step, Page Up/Down use the coarse step, Home/End use limits, and reset uses the declared default even when it differs from the bipolar centre.

### Gestures publish one reversible operation

One pointer drag owns one begin/update/end host gesture and one Undo operation. Typed entry, reset and discrete keyboard or wheel changes each use the same bounded operation path. Escape, pointer cancellation, focus loss during a drag and editor destruction close or cancel ownership exactly once. Intermediate updates remain visible without creating separate history entries.

Direct entry always edits the base value. When modulation is available, the popup distinguishes the base value from the effective value rather than replacing the editable value with live modulation.

### Precision and accessibility are descriptor outputs

Formatting never rewrites an unchanged authoritative value. Each descriptor owns significant digits and step precision so values do not jitter between formats during a gesture. Parsers accept a leading-zero or compact decimal, surrounding whitespace, the real or ASCII minus sign and the descriptor's documented unit placements.

The focusable control retains slider semantics and reports native bounds and a spoken value such as “54 percent left”, “centre” or “minus 12 semitones”. The value editor is keyboard reachable and its invalid state is exposed accessibly. Visible compact notation does not constrain spoken formatting.

## Risks / Trade-offs

- **Rational state can become stale after host automation** → Clear it whenever an authoritative external value no longer matches the rational projection or the descriptor generation changes.
- **Fraction stepping can feel coarse with a small denominator** → Preserve Shift as spatial precision and allow explicit decimal entry to leave rational mode; do not silently change the denominator.
- **Fraction and musical-note syntax can be confused** → Reserve bare fractions for range proportions and require an explicit note suffix in tempo-synced mode.
- **Centralizing descriptors touches many existing controls** → Migrate by variant with focused behavior tests and keep the existing adapter boundary until all callers use descriptors.
- **Compact `.54` formatting may be misread by assistive technology** → Use conventional `0.54` in accessible value text while retaining `.54` visually.
- **Clamping can hide an out-of-range entry** → Keep invalid syntax visibly rejected and show the clamped authoritative value immediately after a valid numeric commit.

## Migration Plan

1. Introduce the descriptor and pure conversion/parser/formatter layer with characterization tests for current parameter units.
2. Add fraction, bipolar, pan, stepped and explicit note-time variants with focused tests before changing rendered controls.
3. Make `NumericField` and `ParameterKnob` consume descriptors and share the edit/gesture state machine.
4. Migrate existing Trigger bindings incrementally while verifying unchanged engine values and host IDs.
5. Expand the catalog to review every variant and interaction at maintained widths and backgrounds.
6. Re-run React/native parity and host-gesture evidence for production-bound controls before syncing the capability into canonical specs.

Rollback retains the current adapters and label-specific formatters until a migrated variant passes its behavior and parity checks.

## Open Questions

Exact visual tick density for each maintained control size remains a catalog design-review choice. Descriptor semantics and labelled major marks are fixed by this change; the review may reduce minor marks to keep compact controls legible.
