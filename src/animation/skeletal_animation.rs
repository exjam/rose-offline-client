use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::LoadState,
    prelude::{
        AssetServer, Assets, Component, Deref, DerefMut, Entity, EventWriter, Handle, Query, Res,
        Transform,
    },
    reflect::Reflect,
    render::mesh::skinning::SkinnedMesh,
    time::Time,
};

use crate::{
    animation::{AnimationFrameEvent, AnimationState, ZmoAsset},
    resources::GameData,
};

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct SkeletalAnimation(AnimationState);

impl SkeletalAnimation {
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self(AnimationState::repeat(motion, limit))
    }

    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self(AnimationState::once(motion))
    }

    pub fn with_animation_speed(mut self, animation_speed: f32) -> Self {
        self.0.set_animation_speed(animation_speed);
        self
    }
}

pub fn skeletal_animation_system(
    mut query_animations: Query<(Entity, &mut SkeletalAnimation, Option<&SkinnedMesh>)>,
    mut query_transform: Query<&mut Transform>,
    mut animation_frame_events: EventWriter<AnimationFrameEvent>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    for (entity, mut skeletal_animation, skinned_mesh) in query_animations.iter_mut() {
        if skeletal_animation.completed() {
            continue;
        }

        let zmo_handle = skeletal_animation.motion();
        let zmo_asset = if let Some(zmo_asset) = motion_assets.get(zmo_handle) {
            zmo_asset
        } else {
            if matches!(
                asset_server.get_load_state(zmo_handle),
                LoadState::Failed | LoadState::Unloaded
            ) {
                // If the asset has failed to load, mark the animation as completed
                skeletal_animation.set_completed();
            }

            continue;
        };

        let animation = &mut skeletal_animation.0;
        animation.advance(zmo_asset, &time);

        animation.iter_animation_events(zmo_asset, |event_id| {
            if let Some(flags) = game_data.animation_event_flags.get(event_id as usize) {
                if !flags.is_empty() {
                    animation_frame_events.send(AnimationFrameEvent::new(entity, *flags));
                }
            }
        });

        let Some(skinned_mesh) = skinned_mesh else {
            continue;
        };
        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();
        let interpolate_weight = animation
            .interpolate_weight()
            .map(|w| (w * FRAC_PI_2).sin());

        for (bone_id, bone_entity) in skinned_mesh.joints.iter().enumerate() {
            let Ok(mut bone_transform) = query_transform.get_mut(*bone_entity) else {
                continue;
            };

            if let Some(translation) = zmo_asset.sample_translation(
                bone_id,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                if let Some(blend_weight) = interpolate_weight {
                    bone_transform.translation =
                        bone_transform.translation.lerp(translation, blend_weight);
                } else {
                    bone_transform.translation = translation;
                }
            }

            if let Some(rotation) = zmo_asset.sample_rotation(
                bone_id,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                if let Some(blend_weight) = interpolate_weight {
                    bone_transform.rotation = bone_transform.rotation.slerp(rotation, blend_weight);
                } else {
                    bone_transform.rotation = rotation;
                }
            }
        }
    }
}
