use bevy::{
    core::Time,
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3},
    prelude::{Commands, Entity, EventWriter, GlobalTransform, Query, Res, Transform},
};

use rose_game_common::components::{Destination, MoveSpeed, Target};

use crate::{
    components::Projectile,
    events::{SpawnEffectData, SpawnEffectEvent},
};

pub fn projectile_system(
    mut commands: Commands,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_bullets: Query<(
        Entity,
        &Projectile,
        &MoveSpeed,
        &Transform,
        Option<&Target>,
        Option<&Destination>,
    )>,
    query_target: Query<&GlobalTransform>,
    time: Res<Time>,
) {
    for (entity, projectile, move_speed, transform, target, destination) in query_bullets.iter() {
        let target_translation = if let Some(target_transform) =
            target.and_then(|target| query_target.get(target.entity).ok())
        {
            target_transform.translation
        } else if let Some(target_position) = destination.map(|destination| destination.position) {
            target_position
        } else {
            // Cannot find target, despawn projectile
            commands.entity(entity).despawn_recursive();
            continue;
        };

        let distance = transform.translation.distance(target_translation);
        let direction = target_translation - transform.translation;
        let move_distance = move_speed.speed * time.delta_seconds();

        if move_distance + 0.1 >= distance {
            // Reached target, play on hit effect
            if let Some(hit_effect_file_id) = projectile.hit_effect_file_id {
                spawn_effect_events.send(SpawnEffectEvent::WithTransform(
                    Transform::from_translation(target_translation),
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
    }
}
