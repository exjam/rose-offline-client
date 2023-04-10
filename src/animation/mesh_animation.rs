use bevy::{
    asset::LoadState,
    prelude::{AssetServer, Assets, Component, Deref, DerefMut, Handle, Query, Res},
    reflect::Reflect,
    time::Time,
};

use crate::{
    animation::{AnimationState, ZmoAsset},
    render::{EffectMeshAnimationFlags, EffectMeshAnimationRenderState},
};

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct MeshAnimation(AnimationState);

impl MeshAnimation {
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self(AnimationState::repeat(motion, limit))
    }

    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self(AnimationState::once(motion))
    }

    pub fn with_start_delay(mut self, start_delay: f32) -> Self {
        self.set_start_delay(start_delay);
        self
    }
}

pub fn mesh_animation_system(
    mut query_animations: Query<(
        &mut MeshAnimation,
        Option<&mut EffectMeshAnimationRenderState>,
    )>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (mut mesh_animation, render_state) in query_animations.iter_mut() {
        if mesh_animation.completed() {
            continue;
        }

        let zmo_handle = mesh_animation.motion();
        let Some(zmo_asset) = motion_assets.get(zmo_handle) else {
            if matches!(asset_server.get_load_state(zmo_handle), LoadState::Failed | LoadState::Unloaded) {
                // If the asset has failed to load, mark the animation as completed
                mesh_animation.set_completed();
            }

            continue;
        };

        let animation = &mut mesh_animation.0;
        animation.advance(zmo_asset, &time);

        let Some(mut render_state) = render_state else {
            continue;
        };
        if let Some(animation_texture) = zmo_asset.animation_texture.as_ref() {
            let mut flags = EffectMeshAnimationFlags::NONE;

            if animation_texture.has_position_channel {
                flags |= EffectMeshAnimationFlags::ANIMATE_POSITION;
            }

            if animation_texture.has_normal_channel {
                flags |= EffectMeshAnimationFlags::ANIMATE_NORMALS;
            }

            if animation_texture.has_uv1_channel {
                flags |= EffectMeshAnimationFlags::ANIMATE_UV;
            }

            if animation_texture.has_alpha_channel {
                flags |= EffectMeshAnimationFlags::ANIMATE_ALPHA;

                let current_alpha = animation_texture.alphas[animation.current_frame_index()];
                let next_alpha = animation_texture.alphas[animation.next_frame_index()];
                render_state.alpha = current_alpha * (1.0 - animation.current_frame_fract())
                    + next_alpha * animation.current_frame_fract();
            }

            render_state.flags = flags.bits() | (zmo_asset.num_frames as u32) << 4;
            render_state.current_next_frame = animation.current_frame_index() as u32
                | (animation.next_frame_index() as u32) << 16;
            render_state.next_weight = animation.current_frame_fract();
        } else {
            render_state.flags = 0;
        }
    }
}
