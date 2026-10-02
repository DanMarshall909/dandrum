# Sampler React WebView

This app is the host integration for the prepared drum sampler. It reads the
copied prepared document and authoritative parameter state, presents real
key/velocity zones, sends editor note events, and consumes the bounded meter
and prepared-waveform services. Round-robin alternatives are shown as one
audition range; prepared map and source structure remain read-only.

Run `npm ci`, `npm test`, `npm run typecheck`, and `npm run build` here after
editing. CMake embeds the checked-in `dist/` assets. The packaged plugin needs
no development server, runtime compiler, or CDN.

The visual `KeyMap`, `LayerStack`, and `OutputBusses` components named in
`openspec/changes/add-renderer-independent-plugin-ui/design.md` are preserved in
`docs/design-system/reference/`. This app currently supplies
the React host integration and capability-aware fallback presentation. Replace
its presentation components with the verified export from that imported reference; retain the tested prepared-data and command lifetimes.
