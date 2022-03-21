use bevy::{
    core::Time,
    prelude::{Assets, Query, Res, Transform},
    render::mesh::skinning::SkinnedMesh,
};

use crate::{components::ActiveMotion, zmo_asset_loader::ZmoAsset};

pub fn animation_system(
    mut query_bones: Query<&mut Transform>,
    mut query_models: Query<(&ActiveMotion, &SkinnedMesh)>,
    motion_assets: Res<Assets<ZmoAsset>>,
    time: Res<Time>,
) {
    let current_time = time.seconds_since_startup();

    for (active_motion, skinned_mesh) in query_models.iter_mut() {
        let current_motion = motion_assets.get(&active_motion.motion);
        if current_motion.is_none() {
            continue;
        }
        let current_motion = current_motion.unwrap();

        let current_animation_time = current_time - active_motion.start_time;
        let current_frame_index_exact = current_animation_time * (current_motion.fps() as f64);
        let current_frame_fract = current_frame_index_exact.fract() as f32;
        let current_frame_index = current_frame_index_exact as usize % current_motion.num_frames();

        // TODO: Support non-looping animations by making next frame optional
        let next_frame_index = (current_frame_index + 1) % current_motion.num_frames();

        for (bone_id, bone_entity) in skinned_mesh.joints.iter().enumerate() {
            if let Ok(mut bone_transform) = query_bones.get_mut(*bone_entity) {
                let current_frame_translation =
                    current_motion.get_translation(bone_id, current_frame_index);
                let next_frame_translation =
                    current_motion.get_translation(bone_id, next_frame_index);

                if let (Some(current_frame_translation), Some(next_frame_translation)) =
                    (current_frame_translation, next_frame_translation)
                {
                    bone_transform.translation =
                        current_frame_translation.lerp(next_frame_translation, current_frame_fract);
                }

                let current_frame_rotation =
                    current_motion.get_rotation(bone_id, current_frame_index);
                let next_frame_rotation = current_motion.get_rotation(bone_id, next_frame_index);
                if let (Some(current_frame_rotation), Some(next_frame_rotation)) =
                    (current_frame_rotation, next_frame_rotation)
                {
                    bone_transform.rotation =
                        current_frame_rotation.lerp(next_frame_rotation, current_frame_fract);
                }
            }
        }
    }
}
