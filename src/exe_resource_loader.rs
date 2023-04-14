use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::{Handle, Image, Vec2},
    reflect::TypeUuid,
    render::texture::{CompressedImageFormats, ImageType},
    utils::BoxedFuture,
};

#[derive(Clone, Default)]
pub struct ExeResourceLoader;

#[derive(Debug, TypeUuid, Clone)]
#[uuid = "dda4ba39-576d-4863-a8b4-ca73cedcfbcd"]
pub struct ExeResourceCursor {
    pub hotspot: Vec2,
    pub size: Vec2,
    pub image: Handle<Image>,
}

impl AssetLoader for ExeResourceLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let file = pelite::PeFile::from_bytes(bytes)?;
            for cursor in file.resources()?.cursors() {
                let Ok((name, cursor)) = cursor else {
                    continue;
                };
                let pelite::resources::Name::Id(id) = name else {
                    continue;
                };

                let mut buffer = Vec::new();
                cursor.write(&mut buffer)?;

                let image = Image::from_buffer(
                    &buffer,
                    ImageType::Extension("ico"),
                    CompressedImageFormats::empty(),
                    false,
                )
                .map_err(|err| {
                    anyhow::anyhow!(
                        "Failed to load cursor {} from {} with error {}",
                        name,
                        load_context.path().display(),
                        err
                    )
                })?;

                let size = image.size();
                let (hotspot_x, hotspot_y) = cursor.hotspot(0).unwrap();
                let hotspot = Vec2::new(hotspot_x as f32, hotspot_y as f32);
                let image_handle = load_context
                    .set_labeled_asset(&format!("cursor_image_{}", id), LoadedAsset::new(image));
                load_context.set_labeled_asset(
                    &format!("cursor_{}", id),
                    LoadedAsset::new(ExeResourceCursor {
                        hotspot,
                        size,
                        image: image_handle,
                    }),
                );
            }

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["exe"]
    }
}
