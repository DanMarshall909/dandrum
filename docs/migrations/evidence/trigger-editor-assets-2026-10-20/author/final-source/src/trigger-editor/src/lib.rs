//! Concrete packaged editor edge. No browser or filesystem policy belongs in the audio host.
#![forbid(unsafe_code)]

mod assets;
pub use assets::{Asset, AssetError, EditorAssets};
