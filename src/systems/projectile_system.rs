use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3},
    prelude::{Commands, Entity, EventWriter, GlobalTransform, Query, Res, Time, Transform},
    render::mesh::skinning::SkinnedMesh,
};

use rose_data::EffectBulletMoveType;
use rose_game_common::components::{Destination, MoveSpeed, Target};

use crate::{
    components::{DummyBoneOffset, Projectile, ProjectileParabola},
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
                .map(|transform| transform.translation())
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

        let (complete, move_vec) = match projectile.move_type {
            EffectBulletMoveType::Linear => {
                let distance = transform.translation.distance(target_translation);
                let direction = target_translation - transform.translation;
                let move_distance = move_speed.speed * time.delta_seconds();

                (
                    move_distance + 0.1 >= distance,
                    move_distance * direction.normalize(),
                )
            }
            EffectBulletMoveType::Parabola => {
                let parabola = projectile.parabola.get_or_insert_with(|| {
                    let distance = transform.translation.distance(target_translation);
                    let travel_time = distance / move_speed.speed;
                    let velocity_y = travel_time * 98.0 / 2.0;

                    let mut move_vec =
                        move_speed.speed * (target_translation - transform.translation).normalize();
                    move_vec.y = velocity_y;

                    ProjectileParabola {
                        start_y: transform.translation.y,
                        end_y: target_translation.y,
                        velocity_y,
                        move_vec,
                        current_time: 0.0,
                        total_time: travel_time,
                    }
                });

                parabola.velocity_y -= 98.0 * time.delta_seconds();
                parabola.move_vec.y = parabola.velocity_y;
                parabola.current_time += time.delta_seconds();

                let mut move_vec = parabola.move_vec * time.delta_seconds();
                move_vec.y += ((parabola.end_y - parabola.start_y) / parabola.total_time)
                    * time.delta_seconds();

                (parabola.current_time >= parabola.total_time, move_vec)
            }
            EffectBulletMoveType::Immediate => (true, Vec3::default()),
        };

        if complete {
            // Reached target, send hit event
            if let Some(target) = target {
                if let Some(skill_id) = projectile.skill_id {
                    hit_events.send(
                        HitEvent::with_skill(projectile.source, target.entity, skill_id)
                            .apply_damage(projectile.apply_damage),
                    );
                } else {
                    hit_events.send(
                        HitEvent::with_weapon(
                            projectile.source,
                            target.entity,
                            projectile.effect_id,
                        )
                        .apply_damage(projectile.apply_damage),
                    );
                }
            }

            commands.entity(entity).despawn_recursive();
            continue;
        }

        // Update transform
        let mut transform = *transform;
        transform.translation += move_vec;
        transform.rotation = Quat::from_rotation_arc(Vec3::X, move_vec.normalize());
        commands.entity(entity).insert(transform);
    }
}
