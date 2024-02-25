use bevy::{
    asset::LoadState,
    prelude::{
        AssetServer, Assets, Component, Deref, DerefMut, Handle, Projection, Query, Res, Transform,
        Vec3,
    },
    reflect::Reflect,
    time::Time,
};

use crate::animation::{AnimationState, ZmoAsset};

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct CameraAnimation(AnimationState);

impl CameraAnimation {
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self(AnimationState::repeat(motion, limit))
    }

    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self(AnimationState::once(motion))
    }
}

pub fn camera_animation_system(
    mut query_animations: Query<(
        &mut CameraAnimation,
        Option<&mut Transform>,
        Option<&mut Projection>,
    )>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (mut camera_animation, transform, projection) in query_animations.iter_mut() {
        if camera_animation.completed() {
            continue;
        }

        let zmo_handle = camera_animation.motion();
        let Some(zmo_asset) = motion_assets.get(zmo_handle) else {
            if matches!(
                asset_server.get_load_state(zmo_handle),
                LoadState::Failed | LoadState::Unloaded
            ) {
                // If the asset has failed to load, mark the animation as completed
                camera_animation.set_completed();
            }

            continue;
        };

        let animation = &mut camera_animation.0;
        animation.advance(zmo_asset, &time);

        let (Some(mut transform), Some(mut projection)) = (transform, projection) else {
            continue;
        };
        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();
        let eye = zmo_asset
            .sample_translation(
                0,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            )
            .map(|eye| eye + Vec3::new(5200.0, 0.0, -5200.0));
        let center = zmo_asset
            .sample_translation(
                1,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            )
            .map(|eye| eye + Vec3::new(5200.0, 0.0, -5200.0));
        let up = zmo_asset.sample_translation(
            2,
            current_frame_fract,
            current_frame_index,
            next_frame_index,
        );
        let fov_near_far = zmo_asset.sample_translation(
            3,
            current_frame_fract,
            current_frame_index,
            next_frame_index,
        );

        if let (Some(eye), Some(center), Some(up)) = (eye, center, up) {
            *transform = Transform::from_translation(eye).looking_at(center, up);
        }

        if let Some(fov_near_far) = fov_near_far {
            if let Projection::Perspective(ref mut perspective_projection) = &mut *projection {
                perspective_projection.fov = (fov_near_far.x * 100.0).to_radians();
                perspective_projection.near = -fov_near_far.z;
                perspective_projection.far = fov_near_far.y * 10.0;
            }
        }
    }
}
