# Production design tokens and fonts

`tokens.json` is the maintained source for production CSS, C++ and Slint values. It was
derived from the five token sheets in `docs/design-system/reference/tokens/` at
`d02ee1b`, whose provenance records the original design archive. Reference files
remain unchanged. Motion constants and the 2-pixel focus gap come from the native
handoff. The reference CSS menu width (240) takes precedence over the old C++
handoff's 248; production dimensions have one authority.

```bash
node scripts/generate-ui-tokens.mjs
node scripts/generate-ui-tokens.mjs --check
node --test tests/js/DesignTokensTest.mjs tests/js/SlintDesignTokensTest.mjs
```

Generated outputs are `web/shared/design-tokens.css`,
`src/juce-plugin/DesignTokens.h`, `ui/slint/tokens.slint` and the preserved
`ui/design-system/DesignTokens.slint`. CSS preserves variable aliases; C++ resolves
them to `inline constexpr` values in `dandrum::ui::tokens`. C++ names replace
hyphens with underscores. These are compile-time constants, similar to C#
`const` fields, and need no JUCE or browser dependency. Sizes are logical pixels;
`em` tracking values are font-size ratios. Colours are packed ARGB integers.
CSS alpha is rounded to the nearest 8-bit channel for the native representation.
Font stacks and shadow descriptions are strings; renderers implement their own
font selection and drawing from those shared values. Slint emits a typed `Tokens`
global: colors, logical lengths, font-size ratios and strings. Native font tokens
select the first locally registered family. Components import it through
`ui/slint/theme.slint`, which registers the original font files.

The Web CTest configuration checks generated-file drift and runs the generator
contract suite, which compiles a native fixture and changes tokens to verify both
outputs. The Slint contract suite additionally checks native aliases, dimensions,
font names, collision rejection and read-only drift detection. A native-only
build consumes checked-in output without requiring Node.

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
and license notices; the files are copied without modification. Both React apps
import `web/shared/design-fonts.css`; Vite inlines the eight TTF files into
`app.css` and retains the three original notices in `app.js`. The embedded
resource provider still serves only HTML, JavaScript and CSS.
[Packaging evidence](../../docs/design-system/react-font-packaging.md) covers
offline Linux WebKit loading and typography. The React apps also compile the
supplied icon geometry through `web/shared/design-icons.mjs`. Native font/icon
embedding is implemented in the [Slint library catalog](../slint/README.md);
integration into the original JUCE editors remains separate. Both original production editors now pass offline
factory/runtime checks at full and compact sizes with their normal host-update
timer; see the packaging evidence for the test boundary and platform limits.

## Slint filter experiment

The filter spike imports the generated `DesignTokens` global. Colour aliases
resolve to Slint RGBA literals; pixel dimensions are lengths, and `em` tracking
remains a font-size ratio. Slint font properties use the first family in each
maintained font stack. CSS and C++ output remain unchanged.

The filter theme imports the retained Barlow Bold, Barlow Semi Condensed SemiBold
and JetBrains Mono Medium files. Its CMake target embeds those files, and both
Standalone and VST3 outputs retain each family's `OFL.txt` under `font-licenses/`
beside the executable/library. This is experimental native packaging, not
completion of the production renderer migration.

Both published Slint import names remain generated from one source: `Tokens` for the full library and `DesignTokens` for the filter/recovered components. `--slint` and `--legacy-slint` select isolated output paths for tests; `--check` validates all four renderer projections.
