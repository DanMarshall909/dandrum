use super::Bundle;

// This deliberately small package supplies an independent painting oracle.
// The separate production-editor lane must still qualify the actual React UI.
pub fn paint_package(bundle: &Bundle) {
    let editor = bundle.resources.join("editor");
    std::fs::create_dir_all(editor.join("fonts")).unwrap();
    std::fs::write(editor.join("index.html"), b"<!doctype html><html><head><link rel='stylesheet' href='/app.css'></head><body><script src='/app.js'></script></body></html>").unwrap();
    std::fs::write(
        editor.join("app.css"),
        b"html,body{margin:0;width:100%;height:100%;background:#123456}",
    )
    .unwrap();
    std::fs::write(
        editor.join("app.js"),
        b"document.title='VST3 painting oracle'",
    )
    .unwrap();
    let fonts =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/trigger/public/fonts");
    for name in [
        "91c841fdfa8e7b94.ttf",
        "c15439e7a03af571.ttf",
        "74103fd6d41bc32c.ttf",
        "c2b046235fb4c2f8.ttf",
        "cb75439ae401f776.ttf",
        "6098b66ce7d608dc.ttf",
        "3386a05f6ece969e.ttf",
        "df54dbfafba61d49.ttf",
        "barlow-OFL.txt",
        "barlowsemicondensed-OFL.txt",
        "jetbrainsmono-OFL.txt",
    ] {
        std::fs::copy(fonts.join(name), editor.join("fonts").join(name)).unwrap();
    }
}
