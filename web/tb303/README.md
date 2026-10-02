# TB-303 React panel

This panel was adapted from `/home/dan/code/tb303-react` at commit
`dd05982ccdc5d7554e032a629c9945be86685670` (the supplied archive
`/home/dan/Downloads/tb303-react.zip`, SHA-256
`30e47b38d24a23f62b9b54a4d24ca651cb8a562294e280c8a46abbb079b187a0`).
The source layout and styling are retained while the mock state is replaced by
the Dandrum host parameter bridge.

Run `npm ci`, `npm run test`, `npm run typecheck`, and `npm run build` here after
editing. The checked-in `dist/` files are embedded by CMake, so building the
plugin from a source release does not require a development server or Node.

The prepared TB-303 patch exposes seven public controls. The saw waveform is
fixed. Pattern editing and transport are unavailable. The keyboard sends real
host note commands; per-note intent, session close and a missed-heartbeat
timeout protect note release when the browser stalls or closes. Host MIDI input
continues to play the instrument independently.
