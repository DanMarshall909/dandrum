## ADDED Requirements

### Requirement: Shared parameter descriptors govern every control presentation

Each maintained parameter control SHALL derive its native range, default, transfer curve, displayed unit, precision, steps, scale marks, parser, formatter and accessible value text from one parameter descriptor shared by rotary and numeric presentations. The descriptor SHALL preserve existing engine values, stable public parameter identities and host automation slots.

#### Scenario: Knob and field share a parameter contract

- **WHEN** the same parameter is rendered as a rotary knob and a numeric field
- **THEN** both controls display, parse, step, clamp and reset the value using the same descriptor
- **AND** equivalent edits publish the same authoritative parameter value

#### Scenario: Nonlinear descriptor maps range positions consistently

- **WHEN** a parameter uses a logarithmic or other nonlinear transfer curve
- **THEN** dragging and normalized range fractions use positions along that curve
- **AND** physical-unit entry resolves through the inverse curve to the same position

### Requirement: Displayed values support direct editing without stealing dial gestures

Clicking a control's displayed value SHALL open its numeric editor, while pointer interaction on the dial SHALL remain a drag gesture. Enter SHALL commit valid input, Escape SHALL cancel, blur SHALL commit only valid input, and double-clicking the dial SHALL restore the declared default.

#### Scenario: Value click opens entry

- **WHEN** the user clicks the displayed value of an enabled parameter control
- **THEN** an editable field opens with the current displayed-unit value selected
- **AND** the click does not begin a dial drag or reset the value

#### Scenario: Dial interaction remains distinct

- **WHEN** the user presses and drags the dial outside the value editor
- **THEN** the parameter follows the drag using its descriptor
- **AND** no text editor opens during that gesture

#### Scenario: Entry commits or cancels predictably

- **WHEN** the user commits valid text with Enter or valid text loses focus
- **THEN** the parsed value is published once
- **WHEN** the user presses Escape or invalid text loses focus
- **THEN** the authoritative value remains unchanged

### Requirement: Percentage and bipolar variants use explicit formats

The system SHALL support unipolar and bipolar percentage controls in both percent and unit-interval display modes. Percent mode SHALL display `54%` for a value of 0.54; unit-interval mode SHALL display `.54` without a leading zero. Bipolar controls SHALL display signed values, draw from a prominent centre origin and use symmetric positive and negative scale marks.

#### Scenario: Unipolar percentage formats are selectable

- **WHEN** a unipolar value of 0.54 uses percent mode
- **THEN** it displays as `54%`
- **WHEN** the same value uses unit-interval mode
- **THEN** it displays as `.54`

#### Scenario: Bipolar control exposes direction and centre

- **WHEN** a bipolar control moves through negative, zero and positive values
- **THEN** it displays signed negative and positive values around `0`
- **AND** its value arc and scale originate at the centre rather than the minimum

### Requirement: Pan presents signed position as left centre or right

A pan descriptor SHALL display negative positions as `L` plus a percentage, zero as `C`, and positive positions as `R` plus a percentage. The displayed percentage SHALL describe pan position independently of the DSP pan law.

#### Scenario: Pan displays direction and amount

- **WHEN** pan positions are -0.54, 0 and 0.54
- **THEN** the control displays `L 54%`, `C` and `R 54%` respectively
- **AND** its accessible text reports “54 percent left”, “centre” and “54 percent right”

#### Scenario: Equivalent pan forms parse identically

- **WHEN** the user enters supported forms such as `L54`, `54% L`, `R 54%` or `C`
- **THEN** each form resolves to the corresponding signed pan position

### Requirement: Fractions represent exact proportions of control range

A signed integer fraction `n/d` with a nonzero positive denominator SHALL represent a normalized proportion of the full control range. A unipolar fraction SHALL resolve from minimum to maximum. A symmetric bipolar fraction SHALL use the sign as its direction from centre and its magnitude as the proportion of that side. The system SHALL retain the entered numerator and denominator for subsequent editing while publishing the descriptor-mapped numeric value.

#### Scenario: Unipolar fraction addresses the full range

- **WHEN** the user enters `1/3` in a unipolar parameter
- **THEN** the control publishes the value at one-third of its normalized travel
- **AND** a nonlinear descriptor applies its transfer curve after locating that normalized position

#### Scenario: Bipolar fractions are symmetric

- **WHEN** the user enters `1/3` and then `-1/3` in a symmetric bipolar parameter
- **THEN** the resulting values are equally distant from centre in positive and negative directions

#### Scenario: Fraction expression survives passive rounding

- **WHEN** `1/3` is committed and the passive display rounds the value to `.333` or `33.3%`
- **THEN** reopening the unchanged editor restores `1/3`
- **AND** the rounded display does not rewrite the authoritative value

### Requirement: Fraction denominators govern subsequent drag steps

While retained fraction state is active, upward and downward value dragging SHALL increment or decrement the retained numerator by one and preserve the entered denominator. Each step SHALL therefore move by `1/d` of normalized range, including when the fraction could be reduced. Shift SHALL increase pointer precision without changing that mathematical step.

#### Scenario: Fraction drag changes the numerator

- **WHEN** a value entered as `1/3` is dragged upward through two value steps
- **THEN** its retained fraction progresses through `2/3` to `3/3`
- **WHEN** it is dragged downward
- **THEN** the numerator decrements while the denominator remains 3

#### Scenario: Unreduced denominator is retained

- **WHEN** the user enters `2/6` and drags upward one value step
- **THEN** the retained fraction becomes `3/6`
- **AND** the step is one-sixth of normalized range rather than one-third

#### Scenario: Decimal entry leaves fraction stepping

- **WHEN** a parameter with retained fraction state receives a valid decimal or physical-unit entry
- **THEN** subsequent dragging uses the descriptor's ordinary configured step
- **AND** the prior denominator no longer controls the gesture

### Requirement: Physical stepped and musical-time variants remain unambiguous

Parameter descriptors SHALL support native-unit time, frequency, pitch, decibel, stepped integer and enumerated values. Discrete descriptors SHALL snap before publication. Bare fractions SHALL always mean range proportions; tempo-synced note values SHALL require explicit notation such as `1/8 note`, `1/8D` or `1/8T`.

#### Scenario: Physical units parse in their displayed domain

- **WHEN** a parameter displays milliseconds, seconds, hertz, kilohertz, semitones, cents or decibels
- **THEN** direct entry in that unit resolves through the descriptor to its authoritative value

#### Scenario: Musical note value is distinguished from a fraction

- **WHEN** a tempo-synced time control receives `1/8D`
- **THEN** it resolves to a dotted eighth-note duration
- **WHEN** any control receives the bare fraction `1/8`
- **THEN** it resolves to one-eighth of normalized parameter range

#### Scenario: Stepped value cannot publish an intermediate

- **WHEN** a semitone, filter-slope, voice-count or enumerated control is dragged or edited between supported steps
- **THEN** it snaps to a supported value before publication
- **AND** it does not display an unsupported intermediate decimal

### Requirement: Validation and formatting preserve authoritative precision

The editor SHALL accept documented compact and conventional decimal forms, surrounding whitespace, ASCII or real minus signs and documented unit placement. Invalid syntax SHALL remain editable with an accessible error. A syntactically valid out-of-range value SHALL clamp on commit and immediately display the resulting authoritative value. Formatting an unchanged value SHALL NOT publish a rounded replacement.

#### Scenario: Equivalent decimal forms are accepted

- **WHEN** a unit-interval descriptor receives `.54`, `0.54` or either form with surrounding whitespace
- **THEN** all forms publish the same value

#### Scenario: Invalid text does not change the parameter

- **WHEN** the user attempts to commit text that the descriptor cannot parse
- **THEN** the editor remains available with an accessible validation error
- **AND** the authoritative parameter and Undo history remain unchanged

#### Scenario: Valid out-of-range text clamps visibly

- **WHEN** valid numeric text lies beyond the descriptor's range and is committed
- **THEN** the value clamps to the applicable limit
- **AND** the editor displays that resulting limit rather than the original text

### Requirement: Parameter gestures are reversible and host-balanced

Each continuous drag SHALL produce one balanced host begin/update/end gesture and one reversible operation. Typed entry, reset and discrete key or wheel edits SHALL use the same bounded operation path. Cancellation, focus loss, pointer loss and editor destruction SHALL close or cancel owned gestures exactly once.

#### Scenario: Drag coalesces into one operation

- **WHEN** a drag produces multiple live value updates
- **THEN** the host receives one balanced gesture around those updates
- **AND** one Undo restores the value held before the drag

#### Scenario: Typed fraction commits once

- **WHEN** the user commits a valid fraction
- **THEN** the mapped value is published in one reversible operation
- **AND** reopening or closing its rounded passive display creates no additional write

#### Scenario: Cancellation restores the prior value

- **WHEN** an active edit or drag is cancelled
- **THEN** the value and rational edit state return to their pre-gesture state
- **AND** host gesture ownership is closed exactly once

### Requirement: Base effective and accessible values remain distinct

Direct manipulation SHALL edit the base parameter value. When modulation data is available, the UI SHALL distinguish the editable base value from the current effective value. Every control SHALL expose keyboard operation, native range information and accessible value text that expands compact visual notation.

#### Scenario: Modulation does not replace editable base value

- **WHEN** a parameter has a base value and a different live effective value
- **THEN** direct entry and dragging change the base value
- **AND** the popup presents the effective value as separate read-only feedback

#### Scenario: Compact notation has spoken meaning

- **WHEN** a visible value uses compact notation such as `.54`, `L 54%` or `−12 st`
- **THEN** assistive technology receives an unambiguous conventional phrase with its leading zero, direction and full unit name

### Requirement: Catalog exercises every maintained parameter variant

The isolated Trigger catalog SHALL expose representative unipolar percentage, unit-interval, bipolar percentage, bipolar unit-interval, pan, pitch, nonlinear frequency, ordinary time, tempo-synced time, decibel and stepped controls. Catalog behavior checks SHALL cover display, direct entry, fraction entry and denominator stepping, dragging, keyboard, reset, validation and accessibility at maintained sizes.

#### Scenario: Variant matrix is reviewable

- **WHEN** a reviewer opens the parameter-control catalog
- **THEN** each maintained variant can be selected with its range, default, unit, transfer and scale marks visible
- **AND** the reviewer does not need to infer one variant from a generic percentage story

#### Scenario: Catalog interactions update controlled values

- **WHEN** a reviewer drags, types, resets or fraction-steps a catalog control
- **THEN** the rendered value, accessible value and Story control show the same accepted result
