use std::{ffi::OsString, num::NonZeroU16, path::PathBuf};

use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    math::{Quat, Vec3},
    prelude::{Handle, Image},
    reflect::{Reflect, TypeUuid},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use rose_file_readers::{RoseFile, ZmoChannel, ZmoFile};

#[derive(Default)]
pub struct ZmoAssetLoader;

#[derive(Default)]
pub struct ZmoTextureAssetLoader;

#[derive(Reflect, Clone, Default)]
pub struct ZmoAssetBone {
    pub translation: Vec<Vec3>,
    pub rotation: Vec<Quat>,
    pub scale: Vec<f32>,
}

#[derive(Reflect, Clone, Default)]
pub struct ZmoAssetAnimationTexture {
    pub texture: Handle<Image>,
    pub alphas: Vec<f32>,
    pub has_position_channel: bool,
    pub has_normal_channel: bool,
    pub has_alpha_channel: bool,
    pub has_uv1_channel: bool,
}

#[derive(Reflect, TypeUuid)]
#[uuid = "120cb5ff-e72d-4730-9756-648d0001fdfa"]
pub struct ZmoAsset {
    pub num_frames: usize,
    pub fps: usize,
    pub frame_events: Vec<u16>,
    pub interpolation_interval: f32,
    pub bones: Vec<ZmoAssetBone>,
    pub animation_texture: Option<ZmoAssetAnimationTexture>,
}

impl ZmoAsset {
    pub fn get_translation(&self, bone_id: usize, frame_id: usize) -> Option<Vec3> {
        self.bones
            .get(bone_id)
            .and_then(|x| x.translation.get(frame_id).cloned())
    }

    pub fn get_rotation(&self, bone_id: usize, frame_id: usize) -> Option<Quat> {
        self.bones
            .get(bone_id)
            .and_then(|x| x.rotation.get(frame_id).cloned())
    }

    pub fn get_scale(&self, bone_id: usize, frame_id: usize) -> Option<f32> {
        self.bones
            .get(bone_id)
            .and_then(|x| x.scale.get(frame_id).cloned())
    }

    pub fn get_frame_event(&self, frame_id: usize) -> Option<NonZeroU16> {
        self.frame_events
            .get(frame_id)
            .and_then(|event_id| NonZeroU16::new(*event_id))
    }

    pub fn sample_translation(
        &self,
        channel_id: usize,
        current_frame_fract: f32,
        current_frame_index: usize,
        next_frame_index: usize,
    ) -> Option<Vec3> {
        let current = self.get_translation(channel_id, current_frame_index);
        let next = self.get_translation(channel_id, next_frame_index);

        if let (Some(current), Some(next)) = (current, next) {
            Some(current.lerp(next, current_frame_fract))
        } else {
            None
        }
    }

    pub fn sample_rotation(
        &self,
        channel_id: usize,
        current_frame_fract: f32,
        current_frame_index: usize,
        next_frame_index: usize,
    ) -> Option<Quat> {
        let current = self.get_rotation(channel_id, current_frame_index);
        let next = self.get_rotation(channel_id, next_frame_index);

        if let (Some(current), Some(next)) = (current, next) {
            Some(current.slerp(next, current_frame_fract))
        } else {
            None
        }
    }

    pub fn sample_scale(
        &self,
        channel_id: usize,
        current_frame_fract: f32,
        current_frame_index: usize,
        next_frame_index: usize,
    ) -> Option<f32> {
        let current = self.get_scale(channel_id, current_frame_index);
        let next = self.get_scale(channel_id, next_frame_index);

        if let (Some(current), Some(next)) = (current, next) {
            Some(current + (next - current) * current_frame_fract)
        } else {
            None
        }
    }
}

impl AssetLoader for ZmoAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            match <ZmoFile as RoseFile>::read(bytes.into(), &Default::default()) {
                Ok(zmo) => {
                    // First count how many transform channels there are
                    let mut max_bone_id = 0;
                    for (bone_id, _) in zmo.channels.iter() {
                        max_bone_id = max_bone_id.max(*bone_id);
                    }

                    // Camera / morph target animations have only position channels
                    // but no bone id so we can use bone id as a channel id instead.
                    let assign_bone_id = max_bone_id == 0 && zmo.channels.len() > 2;
                    if assign_bone_id {
                        max_bone_id = (zmo.channels.len() - 1) as u32;
                    }

                    let mut bones = vec![ZmoAssetBone::default(); (max_bone_id + 1) as usize];
                    for (channel_id, (bone_id, channel)) in zmo.channels.iter().enumerate() {
                        let bone_animation = if !assign_bone_id {
                            &mut bones[*bone_id as usize]
                        } else {
                            &mut bones[channel_id]
                        };
                        match channel {
                            ZmoChannel::Position(positions) => {
                                bone_animation.translation = positions
                                    .iter()
                                    .map(|position| {
                                        Vec3::new(position.x, position.z, -position.y) / 100.0
                                    })
                                    .collect();
                            }
                            ZmoChannel::Rotation(rotations) => {
                                bone_animation.rotation = rotations
                                    .iter()
                                    .map(|rotation| {
                                        Quat::from_xyzw(
                                            rotation.x,
                                            rotation.z,
                                            -rotation.y,
                                            rotation.w,
                                        )
                                    })
                                    .collect();
                            }
                            ZmoChannel::Scale(scales) => {
                                bone_animation.scale = scales.clone();
                            }
                            _ => {}
                        }
                    }
                    load_context.set_default_asset(LoadedAsset::new(ZmoAsset {
                        num_frames: zmo.num_frames,
                        fps: zmo.fps,
                        bones,
                        frame_events: zmo.frame_events,
                        interpolation_interval: (zmo.interpolation_interval_ms.unwrap_or(500)
                            as f32
                            / 1000.0)
                            .max(0.0001),
                        animation_texture: None,
                    }));
                    Ok(())
                }
                Err(error) => Err(error),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["zmo"]
    }
}

impl ZmoTextureAssetLoader {
    pub fn convert_path(path: impl Into<OsString>) -> PathBuf {
        let mut os_string: OsString = path.into();
        os_string.push(".zmo_texture");
        os_string.into()
    }

    pub fn convert_path_texture(path: &str) -> String {
        format!("{}.zmo_texture#image", path)
    }
}

impl AssetLoader for ZmoTextureAssetLoader {
    fn extensions(&self) -> &[&str] {
        &["zmo_texture"]
    }

    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            match <ZmoFile as RoseFile>::read(bytes.into(), &Default::default()) {
                Ok(zmo) => {
                    let mut num_vertices = 0;
                    let mut has_position_channel = false;
                    let mut has_normal_channel = false;
                    let mut has_alpha_channel = false;
                    let mut has_uv1_channel = false;

                    for (vertex_id, channel_type) in zmo.channels.iter() {
                        num_vertices = num_vertices.max(*vertex_id as usize + 1);
                        match channel_type {
                            ZmoChannel::Position(_) => has_position_channel = true,
                            ZmoChannel::Normal(_) => has_normal_channel = true,
                            ZmoChannel::Alpha(_) => has_alpha_channel = true,
                            ZmoChannel::UV1(_) => has_uv1_channel = true,
                            _ => {}
                        }
                    }

                    // RGBA 32F where x = frame number, y = vertex id, pixel = (pos.x, pos.y, pos.z, 0.0)
                    // vert0: (frame[0].pos.xyz, frame[0].uv.x)..n (frame[0].normal.xyz, frame[0].uv.y)..n
                    //  ..
                    // vertN: ..
                    let mut stride = zmo.num_frames;
                    if has_normal_channel || has_uv1_channel {
                        // Use two columns, one for position+uv.x and one for normal+uv.y
                        stride += zmo.num_frames;
                    }

                    let mut image_data = vec![0; num_vertices * stride * 16];
                    let mut alphas = Vec::new();

                    for (vertex_id, channel) in zmo.channels.iter() {
                        match channel {
                            ZmoChannel::Position(values) => {
                                let y = *vertex_id as usize;

                                for (x, position) in values.iter().enumerate() {
                                    let offset = y * stride * 16 + x * 16;

                                    image_data[offset..offset + 4]
                                        .copy_from_slice(&(position.x / 100.0).to_le_bytes());
                                    image_data[offset + 4..offset + 8]
                                        .copy_from_slice(&(position.z / 100.0).to_le_bytes());
                                    image_data[offset + 8..offset + 12]
                                        .copy_from_slice(&(-position.y / 100.0).to_le_bytes());
                                }
                            }
                            ZmoChannel::Normal(values) => {
                                let y = *vertex_id as usize;

                                for (x, normal) in values.iter().enumerate() {
                                    let offset = y * stride * 16 + (zmo.num_frames + x) * 16;

                                    image_data[offset..offset + 4]
                                        .copy_from_slice(&normal.x.to_le_bytes());
                                    image_data[offset + 4..offset + 8]
                                        .copy_from_slice(&normal.z.to_le_bytes());
                                    image_data[offset + 8..offset + 12]
                                        .copy_from_slice(&(-normal.y).to_le_bytes());
                                }
                            }
                            ZmoChannel::UV1(values) => {
                                let y = *vertex_id as usize;

                                for (x, uv) in values.iter().enumerate() {
                                    let offset_uv_x = y * stride * 16 + x * 16;
                                    image_data[offset_uv_x + 12..offset_uv_x + 16]
                                        .copy_from_slice(&uv.x.to_le_bytes());

                                    let offset_uv_y = y * stride * 16 + (zmo.num_frames + x) * 16;
                                    image_data[offset_uv_y + 12..offset_uv_y + 16]
                                        .copy_from_slice(&uv.y.to_le_bytes());
                                }
                            }
                            ZmoChannel::Alpha(values) => {
                                alphas = values.clone();
                            }
                            _ => {}
                        }
                    }

                    let texture_handle = load_context.set_labeled_asset(
                        "image",
                        LoadedAsset::new(Image::new(
                            Extent3d {
                                width: stride as u32,
                                height: num_vertices as u32,
                                depth_or_array_layers: 1,
                            },
                            TextureDimension::D2,
                            image_data,
                            TextureFormat::Rgba32Float,
                        )),
                    );

                    load_context.set_default_asset(LoadedAsset::new(ZmoAsset {
                        num_frames: zmo.num_frames,
                        fps: zmo.fps,
                        frame_events: zmo.frame_events,
                        interpolation_interval: (zmo.interpolation_interval_ms.unwrap_or(500)
                            as f32
                            / 1000.0)
                            .max(0.0001),
                        bones: Vec::new(),
                        animation_texture: Some(ZmoAssetAnimationTexture {
                            texture: texture_handle,
                            alphas,
                            has_position_channel,
                            has_normal_channel,
                            has_alpha_channel,
                            has_uv1_channel,
                        }),
                    }));

                    Ok(())
                }
                Err(error) => Err(error),
            }
        })
    }
}
