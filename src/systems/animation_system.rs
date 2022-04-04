use bevy::{
    core::Time,
    math::Vec3,
    prelude::{Assets, Commands, Entity, PerspectiveProjection, Query, Res, Transform},
    render::{camera::Camera3d, mesh::skinning::SkinnedMesh},
};

use crate::{components::ActiveMotion, zmo_asset_loader::ZmoAsset};

pub fn animation_system(
    mut commands: Commands,
    mut query_transform: Query<&mut Transform>,
    mut query_projection: Query<&mut PerspectiveProjection>,
    mut query_active_motions: Query<(
        Entity,
        &mut ActiveMotion,
        Option<&SkinnedMesh>,
        Option<&Camera3d>,
    )>,
    motion_assets: Res<Assets<ZmoAsset>>,
    time: Res<Time>,
) {
    let current_time = time.seconds_since_startup();

    for (entity, mut active_motion, skinned_mesh, camera_3d) in query_active_motions.iter_mut() {
        let current_motion = motion_assets.get(&active_motion.motion);
        if current_motion.is_none() {
            continue;
        }
        let current_motion = current_motion.unwrap();

        let start_time = if let Some(start_time) = active_motion.start_time {
            start_time
        } else {
            active_motion.start_time = Some(current_time);
            current_time
        };

        let current_frame_index_exact = (current_time - start_time)
            * (current_motion.fps() as f64)
            * active_motion.animation_speed as f64;
        let current_frame_fract = current_frame_index_exact.fract() as f32;
        let current_loop_count = current_frame_index_exact as usize / current_motion.num_frames();
        if current_loop_count >= active_motion.repeat_limit.unwrap_or(usize::MAX) {
            commands.entity(entity).remove::<ActiveMotion>();
            continue;
        }

        let current_frame_index = current_frame_index_exact as usize % current_motion.num_frames();
        let next_frame_index = if current_frame_index + 1 == current_motion.num_frames()
            && current_loop_count + 1 >= active_motion.repeat_limit.unwrap_or(usize::MAX)
        {
            // The last frame of last loop should not blend to the first frame
            current_frame_index
        } else {
            (current_frame_index + 1) % current_motion.num_frames()
        };

        if let Some(skinned_mesh) = skinned_mesh {
            for (bone_id, bone_entity) in skinned_mesh.joints.iter().enumerate() {
                if let Ok(mut bone_transform) = query_transform.get_mut(*bone_entity) {
                    let current_frame_translation =
                        current_motion.get_translation(bone_id, current_frame_index);
                    let next_frame_translation =
                        current_motion.get_translation(bone_id, next_frame_index);

                    if let (Some(current_frame_translation), Some(next_frame_translation)) =
                        (current_frame_translation, next_frame_translation)
                    {
                        bone_transform.translation = current_frame_translation
                            .lerp(next_frame_translation, current_frame_fract);
                    }

                    let current_frame_rotation =
                        current_motion.get_rotation(bone_id, current_frame_index);
                    let next_frame_rotation =
                        current_motion.get_rotation(bone_id, next_frame_index);
                    if let (Some(current_frame_rotation), Some(next_frame_rotation)) =
                        (current_frame_rotation, next_frame_rotation)
                    {
                        bone_transform.rotation =
                            current_frame_rotation.lerp(next_frame_rotation, current_frame_fract);
                    }

                    // TODO: Skinned mesh also support animation of Alpha and UV
                }
            }
        } else if camera_3d.is_some() {
            let current_eye = current_motion.get_translation(0, current_frame_index);
            let next_eye = current_motion.get_translation(0, next_frame_index);
            let eye = if let (Some(current_eye), Some(next_eye)) = (current_eye, next_eye) {
                Some(
                    current_eye.lerp(next_eye, current_frame_fract)
                        + Vec3::new(5200.0, 0.0, -5200.0),
                )
            } else {
                None
            };

            let current_center = current_motion.get_translation(1, current_frame_index);
            let next_center = current_motion.get_translation(1, next_frame_index);
            let center =
                if let (Some(current_center), Some(next_center)) = (current_center, next_center) {
                    Some(
                        current_center.lerp(next_center, current_frame_fract)
                            + Vec3::new(5200.0, 0.0, -5200.0),
                    )
                } else {
                    None
                };

            let current_up = current_motion.get_translation(2, current_frame_index);
            let next_up = current_motion.get_translation(2, next_frame_index);
            let up = if let (Some(current_up), Some(next_up)) = (current_up, next_up) {
                Some(current_up.lerp(next_up, current_frame_fract))
            } else {
                None
            };

            let current_fov_near_far = current_motion.get_translation(3, current_frame_index);
            let next_fov_near_far = current_motion.get_translation(3, next_frame_index);
            let fov_near_far = if let (Some(current_fov_near_far), Some(next_fov_near_far)) =
                (current_fov_near_far, next_fov_near_far)
            {
                Some(current_fov_near_far.lerp(next_fov_near_far, current_frame_fract))
            } else {
                None
            };

            if let (Some(eye), Some(center), Some(up)) = (eye, center, up) {
                if let Ok(mut transform) = query_transform.get_mut(entity) {
                    *transform = Transform::from_translation(eye).looking_at(center, up);
                }
            }

            if let Some(fov_near_far) = fov_near_far {
                if let Ok(mut perspective_projection) = query_projection.get_mut(entity) {
                    perspective_projection.fov = (fov_near_far.x * 100.0).to_radians();
                    perspective_projection.near = -fov_near_far.z;
                    perspective_projection.far = fov_near_far.y * 10.0;
                }
            }
        } else if let Ok(mut entity_transform) = query_transform.get_mut(entity) {
            let current_frame_translation = current_motion.get_translation(0, current_frame_index);
            let next_frame_translation = current_motion.get_translation(0, next_frame_index);

            if let (Some(current_frame_translation), Some(next_frame_translation)) =
                (current_frame_translation, next_frame_translation)
            {
                entity_transform.translation =
                    current_frame_translation.lerp(next_frame_translation, current_frame_fract);
            }

            let current_frame_rotation = current_motion.get_rotation(0, current_frame_index);
            let next_frame_rotation = current_motion.get_rotation(0, next_frame_index);
            if let (Some(current_frame_rotation), Some(next_frame_rotation)) =
                (current_frame_rotation, next_frame_rotation)
            {
                entity_transform.rotation =
                    current_frame_rotation.lerp(next_frame_rotation, current_frame_fract);
            }

            let current_frame_scale = current_motion.get_scale(0, current_frame_index);
            let next_frame_scale = current_motion.get_scale(0, next_frame_index);
            if let (Some(current_frame_scale), Some(next_frame_scale)) =
                (current_frame_scale, next_frame_scale)
            {
                entity_transform.scale = Vec3::splat(
                    current_frame_scale
                        + (next_frame_scale - current_frame_scale) * current_frame_fract,
                );
            }
        }
    }
}
