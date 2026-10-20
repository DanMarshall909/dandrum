use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const FILES: [&str; 14] = [
    "index.html",
    "app.js",
    "app.css",
    "fonts/91c841fdfa8e7b94.ttf",
    "fonts/c15439e7a03af571.ttf",
    "fonts/74103fd6d41bc32c.ttf",
    "fonts/c2b046235fb4c2f8.ttf",
    "fonts/cb75439ae401f776.ttf",
    "fonts/6098b66ce7d608dc.ttf",
    "fonts/3386a05f6ece969e.ttf",
    "fonts/df54dbfafba61d49.ttf",
    "fonts/barlow-OFL.txt",
    "fonts/barlowsemicondensed-OFL.txt",
    "fonts/jetbrainsmono-OFL.txt",
];
static SERIAL: AtomicU64 = AtomicU64::new(0);
pub struct Package {
    root: PathBuf,
}
impl Package {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "dandrum-editor-assets-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("editor")).unwrap();
        Self { root }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.root.join("editor").join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}
impl Drop for Package {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
