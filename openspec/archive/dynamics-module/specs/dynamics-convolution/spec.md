## ADDED Requirements

### Requirement: Convolution module definition
The engine SHALL provide a built-in `convolution` module with `audio_in` (Audio) input, `audio_out` (Audio) output, and a `wet` (Control) input for dry/wet mix. The module SHALL load an impulse response from a declared asset (kind: `impulse_response`) and perform FFT-based partitioned convolution.

#### Scenario: Convolution ports are registered
- **WHEN** querying the built-in registry for the `convolution` module type
- **THEN** the definition SHALL have input ports `audio_in`, `wet` and output port `audio_out`, and parameter `asset`

#### Scenario: Convolution references declared IR asset
- **WHEN** a convolution module parameter `asset` matches an asset declaration with kind `impulse_response`
- **THEN** patch preparation SHALL load the IR WAV file and prepare it for convolution

#### Scenario: Missing IR asset is reported
- **WHEN** a convolution module references an asset that does not exist
- **THEN** patch preparation SHALL fail with a diagnostic identifying the module and missing asset ID

### Requirement: Convolution performs partitioned convolution
The module SHALL implement partitioned convolution using the overlap-add method, processing input audio against the loaded IR in fixed-size partitions.

#### Scenario: Dry signal passes through with no IR
- **WHEN** `wet` is 0%
- **THEN** the convolution output SHALL be the dry input signal (within floating-point precision)

#### Scenario: Impulse input reproduces IR
- **WHEN** the input is a unit impulse and `wet` is 100%
- **THEN** the output SHALL be the loaded IR (within floating-point precision)

#### Scenario: Wet/dry mix blends convolved and dry signal
- **WHEN** `wet` is 50%
- **THEN** the output SHALL be `0.5 * dry + 0.5 * convolved`

### Requirement: Convolution handles edge cases
#### Scenario: No NaN zero input
- **WHEN** the convolution module receives silence
- **THEN** the output SHALL be silence (no NaN or infinite values)

#### Scenario: Short IR loads correctly
- **WHEN** an IR shorter than the partition size (256 samples) is loaded
- **THEN** the module SHALL zero-pad the IR to the partition size and process without error

#### Scenario: Long IR is clamped
- **WHEN** an IR longer than 4 seconds at the project sample rate is loaded
- **THEN** the module SHALL truncate it to 4 seconds and SHALL not error

### Requirement: Deterministic convolution rendering
#### Scenario: Same convolution patch repeats exactly
- **WHEN** the same convolution patch is rendered twice with the same IR and input signal
- **THEN** both renders produce identical audio output
