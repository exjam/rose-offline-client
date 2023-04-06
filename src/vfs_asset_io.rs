use bevy::asset::{AssetIo, AssetIoError, BoxedFuture, Metadata};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rose_file_readers::{VfsFile, VirtualFilesystem};

pub struct VfsAssetIo {
    vfs: Arc<VirtualFilesystem>,
}

impl VfsAssetIo {
    pub fn new(vfs: Arc<VirtualFilesystem>) -> Self {
        Self { vfs }
    }
}

impl AssetIo for VfsAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            // bevy plsssss whyyy
            // HACK: render/texture_array.rs relies on a custom asset loader with extension .image_copy_src
            // HACK: render/rgb_texture_loader.rs relies on a custom asset loader with extension .rgb_texture
            // HACK: zone_loader.rs relies on a custom asset loader with extension .zone_loader
            let path = path
                .to_str()
                .unwrap()
                .trim_end_matches(".image_copy_src")
                .trim_end_matches(".rgb_texture")
                .trim_end_matches(".no_skin")
                .trim_end_matches(".zmo_texture");
            if path.ends_with(".zone_loader") {
                let zone_id = path.trim_end_matches(".zone_loader").parse::<u8>().unwrap();
                Ok(vec![zone_id])
            } else if let Ok(file) = self.vfs.open_file(path) {
                match file {
                    VfsFile::Buffer(buffer) => Ok(buffer),
                    VfsFile::View(view) => Ok(view.into()),
                }
            } else {
                Err(AssetIoError::NotFound(path.into()))
            }
        })
    }

    fn read_directory(
        &self,
        _path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        Ok(Box::new(std::iter::empty::<PathBuf>()))
    }

    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError> {
        Err(AssetIoError::NotFound(path.to_path_buf()))
    }

    fn watch_path_for_changes(
        &self,
        _to_watch: &Path,
        _to_reload: Option<PathBuf>,
    ) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        Ok(())
    }
}
