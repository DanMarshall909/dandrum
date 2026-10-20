//! Concrete packaged editor edge. No browser or filesystem policy belongs in the audio host.
#![forbid(unsafe_code)]

mod assets;
pub use assets::{Asset, AssetError, EditorAssets};

#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_support;

#[cfg(all(target_os = "linux", feature = "native-x11"))]
mod native_x11;
#[cfg(all(target_os = "linux", feature = "native-x11"))]
pub use native_x11::{NativeEditor, NativeError};
