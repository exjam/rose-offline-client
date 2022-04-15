use bevy::{
    core::Time,
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3},
    prelude::{Commands, Entity, EventWriter, Query, Res, Transform},
};

use rose_game_common::components::{MoveSpeed, Target};

use crate::{
    components::Projectile,
    events::{SpawnEffectData, SpawnEffectEvent},
};

pub fn projectile_system(
    mut commands: Commands,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_bullets: Query<(Entity, &Projectile, &MoveSpeed, &Transform, &Target)>,
    query_target: Query<&Transform>,
    time: Res<Time>,
) {
    for (entity, projectile, move_speed, transform, target) in query_bullets.iter() {
        if let Ok(target_transform) = query_target.get(target.entity) {
            let target_position = target_transform.translation + Vec3::new(0.0, 1.0, 0.0);
            let distance = transform.translation.distance(target_position);
            let direction = target_position - transform.translation;
            let move_distance = move_speed.speed * time.delta_seconds();

            if move_distance + 0.1 >= distance {
                // Reached target, play on hit effect
                if let Some(hit_effect_file_id) = projectile.hit_effect_file_id {
                    spawn_effect_events.send(SpawnEffectEvent::AtEntity(
                        target.entity,
                        SpawnEffectData::with_file_id(hit_effect_file_id),
                    ));
                }

                // TODO: Do pending damage / skill effect here
                commands.entity(entity).despawn_recursive();
                continue;
            }

            // Update transform
            let mut transform = *transform;
            transform.translation += move_distance * direction.normalize();
            transform.rotation = Quat::from_axis_angle(
                Vec3::Y,
                direction.z.atan2(direction.x) + std::f32::consts::PI / 2.0,
            );
            commands.entity(entity).insert(transform);
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }
}
