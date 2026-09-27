## 1. Preparation order

- [x] 1.1 Inspect baseline coverage and specify load/prepare order with failing Rust audio tests at 44.1, 48 and 96 kHz.
- [x] 1.2 Make loaded runtime preparation use current host settings, then verify coverage and consider production/test refactoring.

## 2. Repeated preparation and compatibility

- [x] 2.1 Specify and verify changed rate/block size, cleared transient state/events, retained values and usable slot handles.
- [x] 2.2 Add native plugin evidence for restored non-default values, stable host parameter objects and failed replacement recovery at the host sample rate.

## 3. Completion evidence

- [x] 3.1 Run focused and full Rust/native tests, CTest, strict coverage gate, focused mutation analysis and unchanged fixture render comparison; record commands and results.
- [x] 3.2 Obtain independent completion review; resolve findings, synchronize the earned specification and map each new scenario to its proving tests.
- [x] 3.3 Validate OpenSpec and spec coverage, record evidence and commit the reviewed change locally without merging or pushing.
