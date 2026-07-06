## 1. Specification And Validation Surface

- [ ] 1.1 Define the hosted plugin module capability and its patch-level validation rules.
- [ ] 1.2 Specify how hosted plugins are distinguished from built-in modules in the engine model.

## 2. Host Runtime Boundary

- [ ] 2.1 Extend the preparation pipeline to carry hosted plugin metadata and state handles.
- [ ] 2.2 Define how audio, event, and control connections reach a hosted plugin module.
- [ ] 2.3 Define load failure and unsupported plugin behavior before realtime rendering.

## 3. JUCE Integration

- [ ] 3.1 Specify the JUCE-backed plugin loading and discovery layer at the wrapper boundary.
- [ ] 3.2 Specify how plugin state and latency metadata are exposed to the runtime.

## 4. Verification

- [ ] 4.1 Add tests for plugin load failure and preparation-time rejection.
- [ ] 4.2 Add tests for typed audio/event pass-through at the module boundary.
- [ ] 4.3 Run `openspec validate add-plugin-host-module --strict` and fix validation errors.
