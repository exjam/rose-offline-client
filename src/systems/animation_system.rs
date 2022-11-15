use std::f32::consts::FRAC_PI_2;

use bevy::{
    math::{Quat, Vec3},
    prelude::{Assets, Camera3d, Commands, Entity, EventWriter, Query, Res, Time, Transform},
    render::{camera::Projection, mesh::skinning::SkinnedMesh},
};

use crate::{
    components::ActiveMotion, events::AnimationFrameEvent, resources::GameData,
    zmo_asset_loader::ZmoAsset,
};

fn sample_translation(
    zmo_asset: &ZmoAsset,
    channel: usize,
    current_frame_fract: f32,
    current_frame_index: usize,
    next_frame_index: usize,
) -> Option<Vec3> {
    let current = zmo_asset.get_translation(channel, current_frame_index);
    let next = zmo_asset.get_translation(channel, next_frame_index);

    if let (Some(current), Some(next)) = (current, next) {
        Some(current.lerp(next, current_frame_fract))
    } else {
        None
    }
}

fn sample_rotation(
    zmo_asset: &ZmoAsset,
    channel: usize,
    current_frame_fract: f32,
    current_frame_index: usize,
    next_frame_index: usize,
) -> Option<Quat> {
    let current = zmo_asset.get_rotation(channel, current_frame_index);
    let next = zmo_asset.get_rotation(channel, next_frame_index);

    if let (Some(current), Some(next)) = (current, next) {
        Some(current.slerp(next, current_frame_fract))
    } else {
        None
    }
}

fn sample_scale(
    zmo_asset: &ZmoAsset,
    channel: usize,
    current_frame_fract: f32,
    current_frame_index: usize,
    next_frame_index: usize,
) -> Option<f32> {
    let current = zmo_asset.get_scale(channel, current_frame_index);
    let next = zmo_asset.get_scale(channel, next_frame_index);

    if let (Some(current), Some(next)) = (current, next) {
        Some(current + (next - current) * current_frame_fract)
    } else {
        None
    }
}

fn advance_active_motion(
    active_motion: &mut ActiveMotion,
    zmo_asset: &ZmoAsset,
    time: &Time,
) -> Option<(f32, usize, usize)> {
    let current_time = time.elapsed_seconds_f64();
    let start_time = if let Some(start_time) = active_motion.start_time {
        start_time
    } else {
        active_motion.start_time = Some(current_time);
        current_time
    };

    let current_frame_index_exact = (current_time - start_time)
        * (zmo_asset.fps() as f64)
        * active_motion.animation_speed as f64;
    let current_frame_fract = current_frame_index_exact.fract() as f32;
    let current_loop_count = current_frame_index_exact as usize / zmo_asset.num_frames();
    active_motion.loop_count = current_loop_count;
    if current_loop_count >= active_motion.repeat_limit.unwrap_or(usize::MAX) {
        return None; // Animation complete
    }

    let current_frame_index = current_frame_index_exact as usize % zmo_asset.num_frames();
    let next_frame_index = if current_frame_index + 1 == zmo_asset.num_frames()
        && current_loop_count + 1 >= active_motion.repeat_limit.unwrap_or(usize::MAX)
    {
        // The last frame of last loop should not blend to the first frame
        current_frame_index
    } else {
        (current_frame_index + 1) % zmo_asset.num_frames()
    };

    Some((current_frame_fract, current_frame_index, next_frame_index))
}

fn emit_animation_events(
    zmo_asset: &ZmoAsset,
    game_data: &GameData,
    animation_frame_events: &mut EventWriter<AnimationFrameEvent>,
    entity: Entity,
    start_frame_index: usize,
    end_frame_index: usize,
) {
    let mut frame_index = start_frame_index;

    // Emit every frame event between previous frame and current frame
    while frame_index != end_frame_index {
        if let Some(event_id) = zmo_asset.get_frame_event(frame_index) {
            if let Some(flags) = game_data.animation_event_flags.get(event_id.get() as usize) {
                if !flags.is_empty() {
                    animation_frame_events.send(AnimationFrameEvent::new(entity, *flags));
                }
            }
        }

        frame_index = (frame_index + 1) % zmo_asset.num_frames();
    }
}

pub fn animation_system(
    mut commands: Commands,
    mut query_transform: Query<&mut Transform>,
    mut query_projection: Query<&mut Projection>,
    mut query_active_motions: Query<(
        Entity,
        &mut ActiveMotion,
        Option<&SkinnedMesh>,
        Option<&Camera3d>,
    )>,
    mut animation_frame_events: EventWriter<AnimationFrameEvent>,
    game_data: Res<GameData>,
    motion_assets: Res<Assets<ZmoAsset>>,
    time: Res<Time>,
) {
    for (entity, mut active_motion, skinned_mesh, camera_3d) in query_active_motions.iter_mut() {
        let zmo_asset = if let Some(zmo_asset) = motion_assets.get(&active_motion.motion) {
            zmo_asset
        } else {
            continue;
        };

        let (current_frame_fract, current_frame_index, next_frame_index) =
            if let Some(result) = advance_active_motion(&mut active_motion, zmo_asset, &time) {
                result
            } else {
                // Animation complete, emit remaining events and remove ActiveMotion
                emit_animation_events(
                    zmo_asset,
                    &game_data,
                    &mut animation_frame_events,
                    entity,
                    active_motion.previous_frame.unwrap_or(0),
                    zmo_asset.num_frames() - 1,
                );
                commands.entity(entity).remove::<ActiveMotion>();
                continue;
            };

        if active_motion.blend_weight < 1.0 {
            active_motion.blend_weight += time.delta_seconds() / zmo_asset.interpolation_interval();
        }

        let blend_weight = if active_motion.blend_weight < 1.0 {
            Some((active_motion.blend_weight * FRAC_PI_2).sin())
        } else {
            None
        };

        emit_animation_events(
            zmo_asset,
            &game_data,
            &mut animation_frame_events,
            entity,
            active_motion.previous_frame.unwrap_or(0),
            current_frame_index,
        );
        active_motion.previous_frame = Some(current_frame_index);

        if let Some(skinned_mesh) = skinned_mesh {
            for (bone_id, bone_entity) in skinned_mesh.joints.iter().enumerate() {
                if let Ok(mut bone_transform) = query_transform.get_mut(*bone_entity) {
                    if let Some(translation) = sample_translation(
                        zmo_asset,
                        bone_id,
                        current_frame_fract,
                        current_frame_index,
                        next_frame_index,
                    ) {
                        if let Some(blend_weight) = blend_weight {
                            bone_transform.translation =
                                bone_transform.translation.lerp(translation, blend_weight);
                        } else {
                            bone_transform.translation = translation;
                        }
                    }

                    if let Some(rotation) = sample_rotation(
                        zmo_asset,
                        bone_id,
                        current_frame_fract,
                        current_frame_index,
                        next_frame_index,
                    ) {
                        if let Some(blend_weight) = blend_weight {
                            bone_transform.rotation =
                                bone_transform.rotation.slerp(rotation, blend_weight);
                        } else {
                            bone_transform.rotation = rotation;
                        }
                    }
                }
            }
        } else if camera_3d.is_some() {
            let eye = sample_translation(
                zmo_asset,
                0,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            )
            .map(|eye| eye + Vec3::new(5200.0, 0.0, -5200.0));
            let center = sample_translation(
                zmo_asset,
                1,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            )
            .map(|eye| eye + Vec3::new(5200.0, 0.0, -5200.0));
            let up = sample_translation(
                zmo_asset,
                2,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            );
            let fov_near_far = sample_translation(
                zmo_asset,
                3,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            );

            if let (Some(eye), Some(center), Some(up)) = (eye, center, up) {
                if let Ok(mut transform) = query_transform.get_mut(entity) {
                    *transform = Transform::from_translation(eye).looking_at(center, up);
                }
            }

            if let Some(fov_near_far) = fov_near_far {
                if let Ok(mut projection) = query_projection.get_mut(entity) {
                    if let Projection::Perspective(ref mut perspective_projection) =
                        &mut *projection
                    {
                        perspective_projection.fov = (fov_near_far.x * 100.0).to_radians();
                        perspective_projection.near = -fov_near_far.z;
                        perspective_projection.far = fov_near_far.y * 10.0;
                    }
                }
            }
        } else if let Ok(mut entity_transform) = query_transform.get_mut(entity) {
            if let Some(translation) = sample_translation(
                zmo_asset,
                0,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                entity_transform.translation = translation;
            }

            if let Some(rotation) = sample_rotation(
                zmo_asset,
                0,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                entity_transform.rotation = rotation;
            }

            if let Some(scale) = sample_scale(
                zmo_asset,
                0,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                entity_transform.scale = Vec3::splat(scale);
            }

            // TODO: Support animation of alpha
        }

        // TODO: We also need morph targets which supports Vertex Position, Vertex Normal, Vertex UV0-3, Material Alpha
    }
}
