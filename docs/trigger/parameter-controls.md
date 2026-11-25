# Trigger parameter controls

Trigger rotary and numeric controls use the same parameter descriptor from
`web/trigger/shell/parameter-descriptor.mjs`. A descriptor owns the native
range and default, normalized transfer, ordinary/fine/coarse steps, scale
marks, parser, visual formatter and accessible value text. Bindings keep their
existing engine values and public parameter identities; the adapters convert
between native values and the control position.

Maintained variants include percent (`54%`), unit interval (`.54`), bipolar
percent and unit interval, pan (`L 54%`, `C`, `R 54%`), frequency, time, pitch,
cents, decibels, integers, enumerations and tempo-note values. Compact visual
forms have expanded accessible text. Pan percentages describe position and do
not encode a DSP pan law.

A bare signed integer fraction is a proportion of the full control range. For
example, `1/3` selects one third of unipolar travel; on a bipolar descriptor it
selects one third of the positive side from centre. The exact numerator and
denominator remain in edit state, so reopening restores `1/3` even when the
passive display is `33%` or `.33`. Subsequent vertical dragging changes the
numerator by one per step and retains the denominator, including unreduced
forms such as `2/6`. Shift increases the required pointer travel while each
mathematical step remains `1/d`.

Tempo-note input must be explicit: `1/8 note`, `1/8D` and `1/8T` mean ordinary,
dotted and triplet note durations. Bare `1/8` always means one eighth of the
parameter range. Valid out-of-range input clamps to the descriptor limit.
Invalid input remains editable with an accessible error and does not publish a
parameter write.

Click the displayed knob value or a numeric field to type. Enter commits,
Escape cancels, and valid blur commits. Dial pointer input remains a drag;
double-click or the reset shortcut restores the descriptor default. Continuous
drags coalesce under the existing host gesture owner, while typed entry, reset,
keyboard and wheel edits use bounded operations. When modulation telemetry is
present, the editable base value and read-only effective value are shown
separately.

The isolated component catalog exposes variant, range, default, transfer, unit
and scale-mark controls. Use it to inspect direct entry, fraction retention,
denominator stepping, validation, accessibility and compact layout behavior.
