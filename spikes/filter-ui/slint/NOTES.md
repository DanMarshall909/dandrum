# Slint renderer spike notes

This adapter targets Slint 1.18.1, commit
`372cf0ee5577c3dfec309a45e7b778ba4e81b734`, using its full-buffer software
renderer inside JUCE's editor and message loop. There is one platform per linked
runtime and a separate adapter, buffer, generated component and view model for
each editor. A scoped association restores the previous factory target after
component creation, including when creation fails. Slint owns the Paths,
Rectangle gradients, Image, TouchAreas and knob geometry; JUCE presents pixels
and forwards input.

## Integration friction

- The public C++ `WindowAdapter::size()` is non-const even though a header
  example declares it const. `SoftwareRenderer` requires an explicit buffer
  strategy. The adapter uses `NewBuffer` initially for predictable complete
  frames, including exposure and resizing.
- Persistent `VectorModel` instances keep repeated TouchAreas alive during
  gestures; replacing the entire model every frame can cancel a drag.
- Slint RGB pixels are copied explicitly into JUCE `PixelRGB` rather than
  assuming compatible byte/channel layouts. That conversion cost belongs in
  frame measurements.
- Standard `AboutSlint` is displayed unchanged in an accessible About overlay.
- Initial build exposed two integration errors: the image API is the
  `Image(SharedPixelBuffer<Rgb8Pixel>)` constructor, not `load_from_rgb8`, and
  generated C++ lost grouping around a nested ternary selecting toggle state.
  A separate boolean property avoids that ambiguous generated expression.
- Graph height and lower panels derive from window height so all controls remain
  visible at both 1080x760 and 900x700 (minimum 860x680).
- Actual 900x700 capture exposed a fixed-Window binding: resize dispatch alone
  left Slint's root at 1080x760 and cropped its controls. Explicit viewport inputs
  now drive root dimensions from JUCE `resized()` before resize dispatch. The
  interaction check rejects graph/knob/About geometry outside the editor bounds.
- The Slint source build requires Rust >=1.92, C++20, CMake >=3.21 and Corrosion.

## Backend limitations

- Path fill/stroke supports solid brushes in this software implementation;
  gradient Path brushes flatten to a color. Rectangle gradients are supported.
- Paths do not work with `render_by_line()`. This adapter uses `render()`.
- Software shadows, arbitrary transforms and rounded clipping have incomplete
  implementation paths; the design uses supported geometry.
- The initial adapter renders one physical pixel per JUCE logical pixel.
  It has no demonstrated HiDPI rendering or platform-specific deployment
  evidence yet.
- Keyboard forwarding covers printable keys and Escape; comprehensive modifier,
  IME and accessibility bridging is outside this bounded renderer experiment.
- C++ custom platforms expose SoftwareRenderer and SkiaRenderer. A FemtoVG
  fallback requires a Rust/OpenGL bridge and is not the backend measured here.

Build, render, input, multiple-instance/reopen and resource outcomes must be
recorded from actual checks by the integrating controller; source completion
alone does not prove them.

Integrated verification passed all nine owning CTest checks, including actual VST3
host captures, input/host state, two editors and close/reopen. Recorded outcomes
and limits are in the [experiment report](../README.md).

The pinned [upstream licence information](https://github.com/slint-ui/slint/blob/372cf0ee5577c3dfec309a45e7b778ba4e81b734/LICENSE.md)
and licence texts remain in the fetched source tree. This desktop experiment uses
the royalty-free option with the standard visible `AboutSlint` attribution.
