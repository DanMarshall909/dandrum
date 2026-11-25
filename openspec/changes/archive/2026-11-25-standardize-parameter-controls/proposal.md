## Why

Trigger's knobs and numeric fields already support several unit conversions and interaction paths, but those rules are configured independently and expose inconsistent editing, precision and range semantics. A shared parameter-control contract is needed before applying the current redesign feedback across the component catalog and production editor.

## What Changes

- Define one parameter-control model shared by rotary knobs, numeric fields and other continuous controls, including range, transfer curve, default, step, format, parser, scale marks and accessibility text.
- Add direct editing by clicking a displayed value while preserving dial dragging, keyboard, wheel, reset, cancellation and gesture boundaries.
- Add unipolar and bipolar percentage variants in `0–100%` and `0–1` forms, with compact unit-interval formatting such as `.54` and signed bipolar formatting.
- Add exact fraction entry where `n/d` represents a proportion of the full control range and establishes subsequent drag steps of `1/d` while retaining the entered denominator.
- Standardize physical-unit variants including time, tempo-synced note time, frequency, pitch, decibels, pan and stepped integer or enumerated values.
- Define centre-origin presentation for bipolar controls and `L`/`C`/`R` percentage presentation for pan.
- Define validation, precision preservation, host automation, undo, modulation, accessibility and catalog-review behavior for every variant.

## Capabilities

### New Capabilities

- `parameter-control-interaction`: Shared parameter metadata, display, entry, fraction, drag, gesture, scale and accessibility behavior across Trigger controls.

### Modified Capabilities

None.

## Impact

- React control adapters and supplied design-system primitives under `web/trigger/components/` and `web/trigger/vendor/design-system/`.
- Parameter formatting and model projection under `web/trigger/shell/`.
- Component catalog controls and browser coverage under `web/trigger/catalog/` and `web/trigger/tests/`.
- Native and Web host gesture parity where the shared contract reaches production plugin controls.
- Existing normalized engine values and stable host parameter identities remain unchanged; the work changes presentation and interaction semantics around them.
