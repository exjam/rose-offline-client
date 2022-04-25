use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::{AddAsset, App, AssetServer, Handle, Image, Plugin},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_resource::{
            CommandEncoderDescriptor, Extent3d, ImageCopyTexture, Origin3d, Texture, TextureAspect,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
            TextureViewDescriptor,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{TextureError, TextureFormatPixelInfo},
    },
};
use image::DynamicImage;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "f1963cac-7435-4adf-a3cf-676c62f5453f"]
pub struct TextureArray {
    images: Vec<Handle<Image>>,
}

pub struct TextureArrayBuilder {
    image_paths: Vec<String>,
}

impl TextureArrayBuilder {
    pub fn new() -> Self {
        Self {
            image_paths: Vec::new(),
        }
    }

    pub fn add(&mut self, path: String) {
        self.image_paths.push(path);
    }

    pub fn build(self, asset_server: &AssetServer) -> TextureArray {
        let mut images = Vec::new();

        for path in self.image_paths.into_iter() {
            images.push(asset_server.load(&(path + ".image_copy_src")));
        }

        TextureArray { images }
    }
}

pub struct GpuTextureArray {
    pub texture: Texture,
    pub texture_view: TextureView,
}

impl RenderAsset for TextureArray {
    type ExtractedAsset = TextureArray;
    type PreparedAsset = GpuTextureArray;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderAssets<Image>>,
        SRes<RenderQueue>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        texture_array: Self::ExtractedAsset,
        (render_device, gpu_images, render_queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let mut texture_array_gpu_images = Vec::with_capacity(texture_array.images.len());

        for slice in texture_array.images.iter() {
            let gpu_image = gpu_images.get(slice);
            if let Some(gpu_image) = gpu_image {
                texture_array_gpu_images.push(gpu_image);
            } else {
                return Err(PrepareAssetError::RetryNextUpdate(texture_array));
            }
        }

        let size = texture_array_gpu_images[0].size;

        let array_texture = render_device.create_texture(&TextureDescriptor {
            label: Some("texture_array"),
            size: Extent3d {
                width: size.width as u32,
                height: size.height as u32,
                depth_or_array_layers: texture_array_gpu_images.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        });

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("create_texture_array"),
        });

        for (slice, slice_gpu_image) in texture_array_gpu_images.iter().enumerate() {
            command_encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &slice_gpu_image.texture,
                    mip_level: 0,
                    origin: Origin3d { x: 0, y: 0, z: 0 },
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &array_texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: slice as u32,
                    },
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: size.width as u32,
                    height: size.height as u32,
                    depth_or_array_layers: 1,
                },
            );
        }

        let command_buffer = command_encoder.finish();
        render_queue.submit(vec![command_buffer]);

        let texture_view = array_texture.create_view(&TextureViewDescriptor::default());
        Ok(GpuTextureArray {
            texture: array_texture,
            texture_view,
        })
    }
}

#[derive(Default)]
pub struct CopySrcImageAssetLoader;

fn image_to_texture(dyn_img: DynamicImage) -> Image {
    use bevy::core::cast_slice;
    let width;
    let height;

    let data: Vec<u8>;
    let format: TextureFormat;

    match dyn_img {
        DynamicImage::ImageLuma8(i) => {
            let i = DynamicImage::ImageLuma8(i).into_rgba8();
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba8Unorm;

            data = i.into_raw();
        }
        DynamicImage::ImageLumaA8(i) => {
            let i = DynamicImage::ImageLumaA8(i).into_rgba8();
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba8Unorm;

            data = i.into_raw();
        }
        DynamicImage::ImageRgb8(i) => {
            let i = DynamicImage::ImageRgb8(i).into_rgba8();
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba8Unorm;

            data = i.into_raw();
        }
        DynamicImage::ImageRgba8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba8Unorm;

            data = i.into_raw();
        }
        DynamicImage::ImageBgr8(i) => {
            let i = DynamicImage::ImageBgr8(i).into_bgra8();

            width = i.width();
            height = i.height();
            format = TextureFormat::Bgra8Unorm;

            data = i.into_raw();
        }
        DynamicImage::ImageBgra8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Bgra8Unorm;

            data = i.into_raw();
        }
        DynamicImage::ImageLuma16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::R16Uint;

            let raw_data = i.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        DynamicImage::ImageLumaA16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rg16Uint;

            let raw_data = i.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        DynamicImage::ImageRgb16(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::Rgba16Uint;

            let mut local_data =
                Vec::with_capacity(width as usize * height as usize * format.pixel_size());

            for pixel in image.into_raw().chunks_exact(3) {
                // TODO: use the array_chunks method once stabilised
                // https://github.com/rust-lang/rust/issues/74985
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];
                let a = u16::max_value();

                local_data.extend_from_slice(&r.to_ne_bytes());
                local_data.extend_from_slice(&g.to_ne_bytes());
                local_data.extend_from_slice(&b.to_ne_bytes());
                local_data.extend_from_slice(&a.to_ne_bytes());
            }

            data = local_data;
        }
        DynamicImage::ImageRgba16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba16Uint;

            let raw_data = i.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
    }

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
    )
}

impl AssetLoader for CopySrcImageAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut dyn_img = image_to_texture(
                image::load_from_memory_with_format(bytes, image::ImageFormat::Dds)
                    .map_err(TextureError::ImageError)?,
            );
            dyn_img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
            load_context.set_default_asset(LoadedAsset::new(dyn_img));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["image_copy_src"]
    }
}

#[derive(Default)]
pub struct TextureArrayPlugin;

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TextureArray>()
            .add_plugin(ExtractComponentPlugin::<Handle<TextureArray>>::default())
            .add_plugin(RenderAssetPlugin::<TextureArray>::default())
            .init_asset_loader::<CopySrcImageAssetLoader>();
    }
}
