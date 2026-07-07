## 1. Instrument File Watching

- [ ] 1.1 Track the loaded instrument's file path and a cheap change signal (modification time, falling back to a content hash) off the audio thread.
- [ ] 1.2 Add a low-frequency background/message-thread timer that polls the change signal.
- [ ] 1.3 Debounce the change signal so a partially-written file does not trigger a premature reload.
- [ ] 1.4 Add a plugin editor toggle to enable/disable file watching.

## 2. Reload Transaction

- [ ] 2.1 On a detected, stable change, validate and compile the candidate instrument off the audio thread.
- [ ] 2.2 Reuse the existing instrument-replacement transaction (mute, retire old DSP, publish new DSP, unmute) for successful reloads.
- [ ] 2.3 Reconcile existing preset/current parameter values against the new `preset_surface.parameters` on reload.
- [ ] 2.4 Refresh/rebuild the plugin parameter/control surface if the public parameter layout changed.
- [ ] 2.5 On validation/compile/prepare failure, leave the previous DSP running unchanged and report the failure through editor/status reporting.
- [ ] 2.6 Store the reloaded instrument's YAML content in plugin state after a successful file-watch reload, so project recall reflects the edited instrument.

## 3. Tests

- [ ] 3.1 Add tests proving a detected file change triggers a reload only through the explicit replacement path.
- [ ] 3.2 Add tests proving a reload that fails validation/compilation leaves the previous DSP running and reports the failure.
- [ ] 3.3 Add tests proving a partially-written file does not trigger a premature reload.
- [ ] 3.4 Add tests proving disabling file watching stops reloads until re-enabled or a manual reload is requested.
