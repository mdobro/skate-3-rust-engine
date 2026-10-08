//! A plain `std::fs` asset reader. Bevy's Android default reads the APK
//! `AssetManager`, which cannot serve imported game data or mods.
use bevy::asset::io::{AssetReader, AssetReaderError, PathStream, Reader, VecReader};
use bevy::tasks::futures_lite::stream;
use std::path::{Path, PathBuf};

pub(crate) struct StdFsAssetReader {
    root: PathBuf,
}

impl StdFsAssetReader {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

fn error(path: PathBuf, e: std::io::Error) -> AssetReaderError {
    if e.kind() == std::io::ErrorKind::NotFound { AssetReaderError::NotFound(path) } else { e.into() }
}

impl AssetReader for StdFsAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let full = self.root.join(path);
        std::fs::read(&full).map(VecReader::new).map_err(|e| error(full, e))
    }

    /// Sidecar `.meta` files are optional, so they are never present.
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        Err::<VecReader, _>(AssetReaderError::NotFound(self.root.join(path)))
    }

    async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> {
        let full = self.root.join(path);
        let entries = std::fs::read_dir(&full).map_err(|e| error(full, e))?;
        let paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                !name.starts_with('.') && !name.ends_with(".meta")
            })
            .filter_map(|entry| entry.path().strip_prefix(&self.root).ok().map(Path::to_path_buf))
            .collect();
        Ok(Box::new(stream::iter(paths)))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let full = self.root.join(path);
        let meta = std::fs::metadata(&full).map_err(|e| error(full, e))?;
        Ok(meta.is_dir())
    }
}

/// Reader for a local directory: `std::fs` on Android, Bevy's file reader elsewhere.
pub(crate) fn local(root: PathBuf) -> Box<dyn bevy::asset::io::ErasedAssetReader> {
    #[cfg(target_os = "android")]
    return Box::new(StdFsAssetReader::new(root));
    #[cfg(not(target_os = "android"))]
    Box::new(bevy::asset::io::file::FileAssetReader::new(root))
}
