use dandrum_trigger_editor::EditorAssets;
mod common;
use common::{FILES, Package};

#[test]
#[ignore = "production bundle calibration; requires DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR"]
fn copied_production_bundle_serves_all_root_assets_eight_fonts_and_three_notices_without_source_lookup()
 {
    let bundle = std::path::PathBuf::from(
        std::env::var_os("DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR")
            .expect("production bundle input must be configured explicitly"),
    );
    let package = Package::new();
    for path in FILES {
        let bytes = std::fs::read(bundle.join(path))
            .unwrap_or_else(|error| panic!("required production asset {path}: {error}"));
        package.write(path, &bytes);
    }
    let assets = EditorAssets::open(package.root()).unwrap();
    for path in FILES {
        let source = std::fs::read(bundle.join(path))
            .unwrap_or_else(|error| panic!("required production asset {path}: {error}"));
        let response = assets
            .serve(&format!("dandrum://localhost/{path}"))
            .unwrap();
        assert_eq!(response.bytes, source, "copied production bytes for {path}");
        if path.ends_with(".ttf") {
            assert_eq!(response.mime, "font/ttf");
            assert_eq!(&response.bytes[..4], &[0, 1, 0, 0]);
        } else if path.ends_with(".txt") {
            assert_eq!(response.mime, "text/plain; charset=utf-8");
            assert!(
                std::str::from_utf8(response.bytes)
                    .unwrap()
                    .contains("SIL OPEN FONT LICENSE Version 1.1")
            );
        }
    }
    let index = assets.serve("dandrum://localhost/").unwrap();
    assert_eq!(index.mime, "text/html; charset=utf-8");
    assert_eq!(
        index.bytes,
        assets
            .serve("dandrum://localhost/index.html")
            .unwrap()
            .bytes
    );
    let html = std::str::from_utf8(index.bytes).unwrap();
    assert!(html.contains("/app.js") && html.contains("/app.css"));
    let js = assets.serve("dandrum://localhost/app.js").unwrap();
    assert_eq!(js.mime, "text/javascript; charset=utf-8");
    assert!(
        std::str::from_utf8(js.bytes)
            .unwrap()
            .contains("__DANDRUM__")
    );
    let css = assets.serve("dandrum://localhost/app.css").unwrap();
    assert_eq!(css.mime, "text/css; charset=utf-8");
    for path in FILES.into_iter().filter(|path| path.ends_with(".ttf")) {
        assert!(
            std::str::from_utf8(css.bytes)
                .unwrap()
                .contains(&format!("/{path}")),
            "actual CSS references local packaged font {path}"
        );
    }
    // The complete input still exists: a missing copied file must not fall back to it.
    std::fs::remove_file(package.root().join("editor/app.js")).unwrap();
    assert!(EditorAssets::open(package.root()).is_err());
    // The already opened owner has immutable bytes, with no later filesystem reads.
    assert_eq!(
        assets.serve("dandrum://localhost/app.js").unwrap().bytes,
        js.bytes
    );
}
