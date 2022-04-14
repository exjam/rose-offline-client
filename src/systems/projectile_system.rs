use bevy::{
    core::Time,
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3},
    prelude::{AssetServer, Assets, Commands, Entity, Query, Res, ResMut, Transform},
};
use rose_game_common::components::{MoveSpeed, Target};

use crate::{
    components::Projectile,
    effect_loader::spawn_effect,
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::GameData,
    VfsResource,
};

pub fn projectile_system(
    mut commands: Commands,
    query_bullets: Query<(Entity, &Projectile, &MoveSpeed, &Transform, &Target)>,
    query_target: Query<&Transform>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    game_data: Res<GameData>,
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
                if let Some(effect_file_path) =
                    projectile.hit_effect_file_id.and_then(|effect_file_id| {
                        game_data.effect_database.get_effect_file(effect_file_id)
                    })
                {
                    if let Some(effect_entity) = spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        effect_file_path.into(),
                        false,
                    ) {
                        commands
                            .entity(effect_entity)
                            .insert(Transform::from_translation(target_position));
                    }
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
