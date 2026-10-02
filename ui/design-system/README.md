# Production design tokens and fonts

`tokens.json` is the maintained source for production CSS and C++ values. It was
derived from the five token sheets in `docs/design-system/reference/tokens/` at
`d02ee1b`, whose provenance records the original design archive. Reference files
remain unchanged. Motion constants and the 2-pixel focus gap come from the native
handoff. The reference CSS menu width (240) takes precedence over the old C++
handoff's 248; production dimensions have one authority.

```bash
node scripts/generate-ui-tokens.mjs
node scripts/generate-ui-tokens.mjs --check
node --test tests/js/DesignTokensTest.mjs
```

Generated outputs are `web/shared/design-tokens.css` and
`src/juce-plugin/DesignTokens.h`. CSS preserves variable aliases; C++ resolves
them to `inline constexpr` values in `dandrum::ui::tokens`. C++ names replace
hyphens with underscores. These are compile-time constants, similar to C#
`const` fields, and need no JUCE or browser dependency. Sizes are logical pixels;
`em` tracking values are font-size ratios. Colours are packed ARGB integers.
CSS alpha is rounded to the nearest 8-bit channel for the native representation.
Font stacks and shadow descriptions are strings; renderers implement their own
font selection and drawing from those shared values.

The Web CTest configuration checks generated-file drift and runs the generator
contract suite, which compiles a native fixture and changes tokens to verify both
outputs. A native-only build consumes checked-in C++ output without requiring Node.

## Fonts

The unmodified local font binaries and SIL Open Font License 1.1 notices are in
`fonts/`. [Their provenance](fonts/provenance.json) pins source commit, URL,
byte count and SHA-256 for every font and license notice:

| Family | Local styles | Source |
| --- | --- | --- |
| Barlow | Medium, SemiBold, Bold | [Google Fonts](https://github.com/google/fonts/tree/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/barlow) |
| Barlow Semi Condensed | Medium, SemiBold, Bold | [Google Fonts](https://github.com/google/fonts/tree/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/barlowsemicondensed) |
| JetBrains Mono | Medium, SemiBold | [JetBrains](https://github.com/JetBrains/JetBrainsMono/tree/19371302b95d218af43299bce79ddbddd0bc364d/fonts/ttf) |

Keep each family's `OFL.txt` with its distributed binaries. Preserve copyright
and license notices; the files are copied without modification. Font acquisition
does not complete embedding: each renderer still needs local resource packaging
and an offline runtime check under task 3.3. No plugin uses a system-installed or
network font as its authoritative design asset.
