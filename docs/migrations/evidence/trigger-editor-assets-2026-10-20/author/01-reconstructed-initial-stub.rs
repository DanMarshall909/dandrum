use std::path::Path;

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

pub struct EditorAssets;
impl EditorAssets {
    pub fn open(_resource_root: impl AsRef<Path>) -> Result<Self, AssetError> {
        todo!("preload the complete copied production editor package")
    }
    pub fn serve(&self, _uri: &str) -> Result<Asset<'_>, AssetError> {
        todo!("serve only declared local editor URLs")
    }
}
