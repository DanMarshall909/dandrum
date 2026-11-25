## 1. Characterize And Define The Shared Contract

- [ ] 1.1 Add focused characterization tests for every existing Trigger knob and numeric-field formatter, parser, native range, default, step and unchanged-value precision rule.
- [ ] 1.2 Add failing tests for a shared parameter descriptor covering linear and nonlinear transfers, ordinary/fine/coarse steps, scale marks and accessible value text.
- [ ] 1.3 Implement the pure descriptor, conversion, parsing and formatting layer without changing existing control bindings.

## 2. Add Parameter Variants And Fraction Semantics

- [ ] 2.1 Add failing tests and implement unipolar percent, unit-interval, bipolar percent, bipolar unit-interval and `L`/`C`/`R` pan descriptors.
- [ ] 2.2 Add failing tests and implement signed integer-fraction parsing, normalized range mapping, retained numerator/denominator state and invalid-denominator rejection.
- [ ] 2.3 Add failing tests and implement denominator-based drag stepping, including unreduced denominators, symmetric bipolar movement, boundary clamping and Shift spatial precision.
- [ ] 2.4 Add failing tests and implement physical-unit, nonlinear frequency/time, pitch, decibel, stepped and enumerated descriptors.
- [ ] 2.5 Add failing tests and implement explicit tempo-note syntax for ordinary, dotted and triplet values while retaining bare fractions as range proportions.

## 3. Unify Rendered Control Interaction

- [ ] 3.1 Add a failing browser test proving one click on the displayed knob value opens entry while dial pointer-down still begins dragging, then implement the separated hit targets.
- [ ] 3.2 Make `ParameterKnob` and `NumericField` consume the shared descriptor and rational edit state, with Enter, Escape, blur, reset, keyboard, wheel and out-of-range behavior covered before implementation.
- [ ] 3.3 Add accessible validation and expanded spoken values for compact decimals, bipolar values, pan and physical units.
- [ ] 3.4 Prove drag, typed entry, reset, cancellation, focus loss and destruction each produce one reversible operation and balanced host gesture ownership.

## 4. Migrate Trigger Bindings

- [ ] 4.1 Replace label-specific parameter-unit branches with explicit descriptors while preserving accepted values, defaults, public parameter IDs and host slots.
- [ ] 4.2 Migrate envelope, modulator, processor, macro, region and inspector fields one variant at a time with focused regression tests.
- [ ] 4.3 Distinguish editable base values from live effective modulation feedback wherever the owning model supplies both values.

## 5. Expand The Component Catalog

- [ ] 5.1 Add catalog controls for parameter variant, range, default, transfer, unit and scale-mark presets.
- [ ] 5.2 Add browser tests for every maintained variant, direct value editing, fraction reopening, denominator stepping, dragging, keyboard, reset, validation and controlled Story feedback.
- [ ] 5.3 Review scale-mark density, popup placement, focus, errors and compact notation at minimum, default and expanded widths on maintained backgrounds.

## 6. Complete Cross-Renderer Evidence And Documentation

- [ ] 6.1 Run focused changed-owner coverage and mutation evidence for descriptor parsing, gesture state and rational stepping, recording any justified exclusions.
- [ ] 6.2 Re-run maintained React/native knob parity and production host-gesture checks for migrated controls without changing engine or host identities.
- [ ] 6.3 Update the maintained design-system control documentation, map every added scenario to genuine tests in `spec-tests.map`, run strict OpenSpec/spec-coverage validation and complete independent review before publication.
