//! Fixed-origin packaged resources for the concrete editor edge.
//!
//! Preload the complete trusted package before exposing any response. Later
//! requests borrow immutable owned bytes and never join a URL to a filesystem path.

use std::path::Path;

const ORIGIN: &str = "dandrum://localhost";
const REQUIRED: [(&str, &str); 14] = [
    ("index.html", "text/html; charset=utf-8"),
    ("app.js", "text/javascript; charset=utf-8"),
    ("app.css", "text/css; charset=utf-8"),
    ("fonts/91c841fdfa8e7b94.ttf", "font/ttf"),
    ("fonts/c15439e7a03af571.ttf", "font/ttf"),
    ("fonts/74103fd6d41bc32c.ttf", "font/ttf"),
    ("fonts/c2b046235fb4c2f8.ttf", "font/ttf"),
    ("fonts/cb75439ae401f776.ttf", "font/ttf"),
    ("fonts/6098b66ce7d608dc.ttf", "font/ttf"),
    ("fonts/3386a05f6ece969e.ttf", "font/ttf"),
    ("fonts/df54dbfafba61d49.ttf", "font/ttf"),
    ("fonts/barlow-OFL.txt", "text/plain; charset=utf-8"),
    (
        "fonts/barlowsemicondensed-OFL.txt",
        "text/plain; charset=utf-8",
    ),
    ("fonts/jetbrainsmono-OFL.txt", "text/plain; charset=utf-8"),
];

#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    InvalidRoot,
    InvalidPackage(&'static str),
    Unreadable(&'static str),
    UnknownUri,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Asset<'a> {
    pub mime: &'static str,
    pub bytes: &'a [u8],
}

struct StoredAsset {
    path: &'static str,
    mime: &'static str,
    bytes: Vec<u8>,
}

pub struct EditorAssets {
    assets: Vec<StoredAsset>,
}
impl EditorAssets {
    /// Open only this explicit resource root's editor subtree, with no fallback.
    /// A missing, empty, linked or unreadable required file rejects the package.
    pub fn open(resource_root: impl AsRef<Path>) -> Result<Self, AssetError> {
        let supplied = resource_root.as_ref();
        let root = supplied
            .canonicalize()
            .map_err(|_| AssetError::InvalidRoot)?;
        let editor = root.join("editor");
        if supplied.is_symlink() || !root.is_dir() || editor.is_symlink() || !editor.is_dir() {
            return Err(AssetError::InvalidRoot);
        }
        let mut assets = Vec::with_capacity(REQUIRED.len());
        for (relative, mime) in REQUIRED {
            let mut path = editor.clone();
            for component in relative.split('/') {
                path.push(component);
                if path.is_symlink() {
                    return Err(AssetError::InvalidPackage(relative));
                }
            }
            let canonical = path
                .canonicalize()
                .map_err(|_| AssetError::InvalidPackage(relative))?;
            if !canonical.starts_with(&editor) || !canonical.is_file() {
                return Err(AssetError::InvalidPackage(relative));
            }
            let bytes = std::fs::read(canonical).map_err(|_| AssetError::Unreadable(relative))?;
            if bytes.is_empty() {
                return Err(AssetError::InvalidPackage(relative));
            }
            assets.push(StoredAsset {
                path: relative,
                mime,
                bytes,
            });
        }
        Ok(Self { assets })
    }

    /// Resolve an exact declared local URL. Queries, fragments, encoded names,
    /// alternate authorities and arbitrary paths do not expand this capability.
    pub fn serve(&self, uri: &str) -> Result<Asset<'_>, AssetError> {
        let path = uri.strip_prefix(ORIGIN).ok_or(AssetError::UnknownUri)?;
        let relative = if path == "/" {
            "index.html"
        } else {
            path.strip_prefix('/').ok_or(AssetError::UnknownUri)?
        };
        let asset = self
            .assets
            .iter()
            .find(|asset| asset.path == relative)
            .ok_or(AssetError::UnknownUri)?;
        Ok(Asset {
            mime: asset.mime,
            bytes: &asset.bytes,
        })
    }
}

#[cfg(test)]
#[path = "../tests/internal/assets.rs"]
mod tests;
