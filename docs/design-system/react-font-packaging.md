# React font and icon packaging

The sampler and TB-303 React apps embed the eight pinned font files from
[font provenance](../../ui/design-system/fonts/provenance.json). They use the
maintained typography aliases: Barlow Semi Condensed for UI labels, JetBrains
Mono for values, and Barlow for the sampler title. Colour, spacing and component
adaptation to the supplied design remain pending.

The apps also use the supplied stroke icons for existing headings, alternate
and choke labels, note audition and error messages. Text labels retain their
meaning; decorative icons do not become separate keyboard targets.

## Assets

[design-fonts.css](../../web/shared/design-fonts.css) declares the local faces.
Both Vite configurations inline them into `app.css`; no font route or font
server is required. [font-licenses.mjs](../../web/shared/font-licenses.mjs)
prepares the unmodified OFL notices for inclusion in each executable JS bundle.
The production resource provider continues to serve three files: `index.html`,
`app.js` and `app.css`.

[design-icons.mjs](../../web/shared/design-icons.mjs) adapts the supplied
[Icon component](reference/components/icons/Icon.jsx) into shared SVG properties.
All 33 names and shape attributes match the original SVG files. Icons inherit
the surrounding text colour; headings use 16 pixels, the TB-303 uses 14 pixels
and compact alternate/choke labels use 12 pixels. The imported reference files
are unchanged. Geometry is compiled into JavaScript, with no additional route
or external SVG request. Decorative icons are hidden from accessible names and
cannot take focus; standalone icons can expose an accessible label.

```bash
npm ci --prefix web/sampler
npm ci --prefix web/tb303
npm run build --prefix web/sampler
npm run build --prefix web/tb303
node --test tests/js/ReactAssetsTest.mjs
node --test tests/js/DesignIconsTest.mjs
cmake --build build
ctest --test-dir build --output-on-failure
```

The root CMake/CTest workflow requires the sampler's locked React dependencies
for the static-rendering oracle. CMake reports the `npm ci --prefix web/sampler`
setup command before configuring JUCE if those dependencies cannot load.
Verification hid the installed package directory in a private mount namespace
and confirmed that diagnostic, then confirmed normal configuration with the
directory restored. A fresh copy of the package and lockfile also passed
`npm ci` using the local cache and rendered an SVG with the actual React server
renderer. Native-only configuration bypasses the check.

The packaging suite compares each decoded CSS font with its pinned SHA-256,
checks family/weight associations, declared font-token references and unchanged
notices, and rejects external
CSS imports, CDN scripts, runtime Babel and extra asset routes. Changing a
Barlow face from weight 500 to 400 produced the named `Barlow:400` failure;
restoring the bytes passed. The title's undefined `--font-title` reference also
failed by name; using the existing `--font-display` token passed. A focused V8
run covered all executable ranges of
the license-banner module.

The icon rendering suite uses React's actual static renderer and compares every
path, circle and rectangle with the supplied SVG files. It also checks sizes,
colour inheritance and decorative/labelled accessibility. A changed `level`
path failed by name, and restoring it passed. The shared icon module's executable
ranges were all covered in the focused V8 run. Packaging tests check that both
compiled apps contain their required supplied geometry.

## Runtime evidence — Linux WebKit

`tb303-web-runtime` and `sampler-web-runtime` instantiate actual processors and
reuse the production editor's embedded resource provider and native function
options. A separate visible test browser reports through an added test-only
native function. The fixture publishes shared bridge parameter updates to that
browser. This checks the compiled app and adapter; it does not exercise the
original editor widget's complete lifecycle or a DAW.

Each check:

- Waits for font readiness and loads all eight declared faces.
- Obtains the real seven TB-303 or 23 sampler public parameters.
- Verifies the UI/value font families and absence of alert errors.
- Verifies visible supplied icons with the expected identities, view box,
  declared size, stroke, decorative accessibility and no keyboard focus.
- Observes a processor parameter change to 0.37 in the rendered control.
- Checks horizontal bounds at 1200×800 and 820×560 after React layout settles.
- Checks that the compact TB-303 panel leaves its hint visible below it.

The hint check failed with the fixed 690-pixel frame. The frame now measures
the panel's intrinsic height through its existing resize observer, including
changes caused by font loading. The regression and subsequent screenshots pass.

Two copied ELF executables also passed with both source checkouts hidden by
private bind mounts, fresh redirected application-data directories, and no
network connectivity in a private network namespace. Their SHA-256 values
matched the built executables before launch. The font load reports, zero exits,
and screenshots were retained together. No development server was used.

Full Web CTest passed **40/40**, including both runtime checks without skips.
Native-only CTest passed **18/18** with browser support disabled and its Node
path set to `/nonexistent/node`. A missing X display produces an explicit CTest
skip (77); that is not runtime evidence.

The headless run used Xvfb with xfwm4 and software Mesa rendering. This machine's
NVIDIA/EGL cleanup crashed during the initial diagnostic runs; selecting Mesa's
EGL vendor avoided that teardown failure:

```bash
DISPLAY=:98 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json \
ctest --test-dir build -L runtime --output-on-failure
```

Vendored Linux JUCE also truncates Unicode JSON in its IPC framing: it counts
characters before copying UTF-8 bytes. An early evaluation diagnostic containing
Unicode UI text was dropped and shifted its callback FIFO. The final harness
uses ASCII report fields and native Promise IDs. Vendored JUCE was not changed;
Unicode bridge support remains an unresolved limitation.

This evidence covers Linux WebKit font/icon loading and the shared parameter
adapter. The original production editor's offline opening remains to verify
before task 3.3 is closed. Native font/icon embedding, other WebView platforms, DAW audio/transport,
meter delivery, note focus, reload ownership, supplied component fidelity and
automatic structural rebuilding still require their owning implementation gates.
