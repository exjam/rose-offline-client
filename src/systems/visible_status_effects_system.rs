use bevy::{
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    prelude::{AssetServer, Assets, Changed, Commands, Entity, Query, Res, ResMut},
};
use rose_game_common::components::StatusEffects;

use crate::{
    components::VisibleStatusEffects,
    effect_loader::spawn_effect,
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::GameData,
    VfsResource,
};

pub fn visible_status_effects_system(
    mut commands: Commands,
    mut query_status_effects: Query<
        (Entity, &StatusEffects, &mut VisibleStatusEffects),
        Changed<StatusEffects>,
    >,
    game_data: Res<GameData>,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
) {
    for (entity, status_effects, mut visible_status_effects) in query_status_effects.iter_mut() {
        for (effect_type, active_status_effect) in status_effects.active.iter() {
            let visible_status_effect = &mut visible_status_effects.effects[effect_type];

            if let Some(active_status_effect) = active_status_effect {
                if let Some((visible_status_effect_id, visible_status_effect_entity)) =
                    visible_status_effect.as_ref()
                {
                    if *visible_status_effect_id == active_status_effect.id {
                        continue;
                    }

                    commands
                        .entity(*visible_status_effect_entity)
                        .despawn_recursive();
                    *visible_status_effect = None;
                }

                if let Some(status_effect_data) = game_data
                    .status_effects
                    .get_status_effect(active_status_effect.id)
                {
                    if let Some(effect_path) = status_effect_data
                        .effect_id
                        .and_then(|effect_id| game_data.effect_database.get_effect(effect_id))
                    {
                        if let Some(effect_entity) = spawn_effect(
                            &vfs_resource.vfs,
                            &mut commands,
                            &asset_server,
                            particle_materials.as_mut(),
                            effect_mesh_materials.as_mut(),
                            effect_path.into(),
                            true,
                        ) {
                            commands.entity(entity).add_child(effect_entity);
                            *visible_status_effect = Some((active_status_effect.id, effect_entity));
                        }
                    }
                }
            } else if let Some((_, visible_status_effect_entity)) = visible_status_effect.take() {
                commands
                    .entity(visible_status_effect_entity)
                    .despawn_recursive();
            }
        }
    }
}
