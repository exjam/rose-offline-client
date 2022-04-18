use bevy::{
    core::Time,
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3},
    prelude::{Commands, Entity, EventWriter, GlobalTransform, Query, Res, Transform},
    render::mesh::skinning::SkinnedMesh,
};

use rose_data::EffectBulletMoveType;
use rose_game_common::components::{Destination, MoveSpeed, Target};

use crate::{
    components::{DummyBoneOffset, Projectile},
    events::HitEvent,
};

pub fn projectile_system(
    mut commands: Commands,
    mut hit_events: EventWriter<HitEvent>,
    mut query_bullets: Query<(
        Entity,
        &mut Projectile,
        &MoveSpeed,
        &Transform,
        Option<&Target>,
        Option<&Destination>,
    )>,
    query_global_transform: Query<&GlobalTransform>,
    query_skeleton: Query<(&SkinnedMesh, &DummyBoneOffset)>,
    time: Res<Time>,
) {
    for (entity, mut projectile, move_speed, transform, target, destination) in
        query_bullets.iter_mut()
    {
        let target_translation = if let Some(target) = target {
            query_skeleton
                .get(target.entity)
                .ok()
                .map(|(skinned_mesh, dummy_bone_offset)| {
                    if dummy_bone_offset.index > 0 {
                        skinned_mesh.joints.last().copied().unwrap_or(target.entity)
                    } else {
                        target.entity
                    }
                })
                .and_then(|target_entity| query_global_transform.get(target_entity).ok())
                .map(|transform| transform.translation)
        } else {
            destination.map(|destination| destination.position)
        };

        if target_translation.is_none() {
            // Cannot find target, despawn projectile
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let mut target_translation = target_translation.unwrap();
        target_translation.y += 0.5;

        let distance = transform.translation.distance(target_translation);
        let direction = target_translation - transform.translation;
        let move_distance = move_speed.speed * time.delta_seconds();

        if matches!(projectile.move_type, EffectBulletMoveType::Parabola) {
            if let Some(parabola_velocity) = projectile.parabola_velocity.as_mut() {
                *parabola_velocity -= 0.98 * time.delta_seconds();
            } else {
                let travel_time = distance / move_speed.speed;
                projectile.parabola_velocity = Some(travel_time * 0.98);
            }
        }

        if move_distance + 0.1 >= distance {
            // Reached target, send hit event
            if let Some(target) = target {
                if let Some(skill_id) = projectile.skill_id {
                    hit_events.send(HitEvent::with_skill(
                        projectile.source,
                        target.entity,
                        skill_id,
                    ));
                } else {
                    hit_events.send(HitEvent::with_weapon(
                        projectile.source,
                        target.entity,
                        projectile.effect_id,
                    ));
                }
            }

            commands.entity(entity).despawn_recursive();
            continue;
        }

        // Update transform
        let mut transform = *transform;
        transform.translation += move_distance * direction.normalize();
        transform.rotation = Quat::from_axis_angle(Vec3::Y, (-direction.z).atan2(direction.x));

        if let Some(parabola_velocity) = projectile.parabola_velocity.as_ref() {
            transform.translation.y += parabola_velocity;
        }

        commands.entity(entity).insert(transform);
    }
}
