use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    prelude::{FromWorld, Image, World},
    render::{
        renderer::RenderDevice,
        texture::{CompressedImageFormats, ImageType, TextureError},
    },
};
use thiserror::Error;

#[derive(Clone)]
pub struct RgbTextureLoader {
    supported_compressed_formats: CompressedImageFormats,
}

impl RgbTextureLoader {
    pub fn convert_path(path: &Path) -> PathBuf {
        let mut os_string: OsString = path.into();
        os_string.push(".rgb_texture");
        os_string.into()
    }
}

impl AssetLoader for RgbTextureLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            // use the file extension for the image type
            let original_path = load_context.path().with_extension("");
            let ext = original_path.extension().unwrap().to_str().unwrap();

            let dyn_img = Image::from_buffer(
                bytes,
                ImageType::Extension(ext),
                self.supported_compressed_formats,
                false,
            )
            .map_err(|err| FileTextureError {
                error: err,
                path: format!("{}", load_context.path().display()),
            })?;

            load_context.set_default_asset(LoadedAsset::new(dyn_img));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["rgb_texture"]
    }
}

impl FromWorld for RgbTextureLoader {
    fn from_world(world: &mut World) -> Self {
        let supported_compressed_formats = match world.get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),

            None => CompressedImageFormats::all(),
        };
        Self {
            supported_compressed_formats,
        }
    }
}

/// An error that occurs when loading a texture from a file.
#[derive(Error, Debug)]
pub struct FileTextureError {
    error: TextureError,
    path: String,
}
impl std::fmt::Display for FileTextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Error reading image file {}: {}, this is an error in `bevy_render`.",
            self.path, self.error
        )
    }
}
