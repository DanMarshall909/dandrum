use super::*;
use crate::test_support::{FILES, Package};

fn fixture() -> Package {
    let package = Package::new();
    for name in FILES {
        package.write(name, format!("packaged:{name}").as_bytes());
    }
    package
}
#[test]
fn immutable_copied_assets_serve_only_the_exact_local_origin_and_declared_names() {
    let package = fixture();
    package.write("fonts/.gitattributes", b"not a browser asset");
    let assets = EditorAssets::open(package.root()).unwrap();
    for path in FILES {
        let asset = assets
            .serve(&format!("dandrum://localhost/{path}"))
            .unwrap();
        assert_eq!(asset.bytes, format!("packaged:{path}").as_bytes());
        let mime = if path.ends_with(".ttf") {
            "font/ttf"
        } else if path.ends_with(".txt") {
            "text/plain; charset=utf-8"
        } else {
            match path {
                "index.html" => "text/html; charset=utf-8",
                "app.js" => "text/javascript; charset=utf-8",
                "app.css" => "text/css; charset=utf-8",
                _ => unreachable!(),
            }
        };
        assert_eq!(asset.mime, mime, "declared type for {path}");
    }
    assert_eq!(
        assets.serve("dandrum://localhost/").unwrap(),
        assets.serve("dandrum://localhost/index.html").unwrap()
    );
    for uri in [
        "/",
        "app.js",
        "https://localhost/app.js",
        "http://localhost/app.js",
        "file:///app.js",
        "dandrum://localhost.evil/app.js",
        "dandrum://127.0.0.1/app.js",
        "dandrum://localhost:80/app.js",
        "dandrum://localhost@evil/app.js",
        "dandrum://localhost/app.js?path=elsewhere",
        "dandrum://localhost/app.js#fragment",
        "dandrum://localhost/%61pp.js",
        "dandrum://localhost/../instrument/advanced-drum-kit.yaml",
        "dandrum://localhost/fonts/.gitattributes",
        "dandrum://localhost/fonts/unknown.ttf",
        "dandrum://localhost//app.js",
        "dandrum://localhost",
    ] {
        assert_eq!(
            assets.serve(uri),
            Err(AssetError::UnknownUri),
            "fixed local URL policy for {uri}"
        );
    }
    std::fs::remove_file(package.root().join("editor/app.js")).unwrap();
    assert_eq!(
        assets.serve("dandrum://localhost/app.js").unwrap().bytes,
        b"packaged:app.js"
    );
    assert!(matches!(
        EditorAssets::open(package.root()),
        Err(AssetError::InvalidPackage("app.js"))
    ));
}
#[test]
fn missing_empty_and_nonregular_package_files_never_publish_a_partial_owner() {
    let missing = Package::new();
    assert!(matches!(
        EditorAssets::open(missing.root().join("missing")),
        Err(AssetError::InvalidRoot)
    ));
    std::fs::write(missing.root().join("root-file"), b"file").unwrap();
    assert!(matches!(
        EditorAssets::open(missing.root().join("root-file")),
        Err(AssetError::InvalidRoot)
    ));
    std::fs::remove_dir(missing.root().join("editor")).unwrap();
    assert!(matches!(
        EditorAssets::open(missing.root()),
        Err(AssetError::InvalidRoot)
    ));
    std::fs::write(missing.root().join("editor"), b"file").unwrap();
    assert!(matches!(
        EditorAssets::open(missing.root()),
        Err(AssetError::InvalidRoot)
    ));
    for operation in 0..3 {
        let package = fixture();
        let path = package.root().join("editor/app.css");
        match operation {
            0 => std::fs::remove_file(&path).unwrap(),
            1 => std::fs::write(&path, b"").unwrap(),
            _ => {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            }
        }
        assert!(matches!(
            EditorAssets::open(package.root()),
            Err(AssetError::InvalidPackage("app.css"))
        ));
    }
    let package = fixture();
    std::fs::remove_file(package.root().join("editor/fonts/df54dbfafba61d49.ttf")).unwrap();
    assert!(matches!(
        EditorAssets::open(package.root()),
        Err(AssetError::InvalidPackage("fonts/df54dbfafba61d49.ttf"))
    ));
}
#[cfg(unix)]
#[test]
fn symlinked_roots_editor_directories_files_and_font_directories_are_rejected() {
    let package = fixture();
    let other = fixture();
    let root_link = package.root().join("root-link");
    std::os::unix::fs::symlink(other.root(), &root_link).unwrap();
    assert!(matches!(
        EditorAssets::open(&root_link),
        Err(AssetError::InvalidRoot)
    ));
    let editor = package.root().join("editor");
    std::fs::rename(&editor, package.root().join("original-editor")).unwrap();
    std::os::unix::fs::symlink(other.root().join("editor"), &editor).unwrap();
    assert!(matches!(
        EditorAssets::open(package.root()),
        Err(AssetError::InvalidRoot)
    ));
    std::fs::remove_file(&editor).unwrap();
    std::fs::rename(package.root().join("original-editor"), &editor).unwrap();
    let file = editor.join("app.js");
    std::fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink(editor.join("app.css"), &file).unwrap();
    assert!(matches!(
        EditorAssets::open(package.root()),
        Err(AssetError::InvalidPackage("app.js"))
    ));
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, b"local-script").unwrap();
    std::fs::rename(editor.join("fonts"), editor.join("original-fonts")).unwrap();
    std::os::unix::fs::symlink(editor.join("original-fonts"), editor.join("fonts")).unwrap();
    assert!(matches!(
        EditorAssets::open(package.root()),
        Err(AssetError::InvalidPackage("fonts/91c841fdfa8e7b94.ttf"))
    ));
}
#[cfg(unix)]
#[test]
fn unreadable_assets_fail_preload_and_do_not_affect_an_existing_immutable_owner() {
    use std::os::unix::fs::PermissionsExt;
    let package = fixture();
    let assets = EditorAssets::open(package.root()).unwrap();
    let path = package.root().join("editor/app.js");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let result = EditorAssets::open(package.root());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(result, Err(AssetError::Unreadable("app.js"))));
    assert_eq!(
        assets.serve("dandrum://localhost/app.js").unwrap().bytes,
        b"packaged:app.js"
    );
}
