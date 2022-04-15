use bevy::{
    hierarchy::BuildChildren,
    prelude::{
        AssetServer, Assets, Commands, EventReader, GlobalTransform, Query, Res, ResMut, Transform,
    },
    render::mesh::skinning::SkinnedMesh,
};
use rose_file_readers::VfsPath;

use crate::{
    components::{CharacterModel, NpcModel},
    effect_loader::spawn_effect,
    events::{SpawnEffect, SpawnEffectData, SpawnEffectEvent},
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::GameData,
    VfsResource,
};

fn get_effect_file_path<'a>(
    spawn_effect_data: &'a SpawnEffectData,
    game_data: &'a GameData,
) -> Option<VfsPath<'a>> {
    match &spawn_effect_data.effect {
        SpawnEffect::FileId(file_id) => game_data
            .effect_database
            .get_effect_file(*file_id)
            .map(|x| x.into()),
        SpawnEffect::Path(path) => Some(path.into()),
    }
}

pub fn spawn_effect_system(
    mut commands: Commands,
    mut events: EventReader<SpawnEffectEvent>,
    query_transform: Query<&GlobalTransform>,
    query_skeleton: Query<(&SkinnedMesh, Option<&CharacterModel>, Option<&NpcModel>)>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
) {
    for event in events.iter() {
        match event {
            SpawnEffectEvent::InEntity(effect_entity, spawn_effect_data) => {
                if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data)
                {
                    spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        effect_file_path,
                        false,
                        Some(*effect_entity),
                    );
                }
            }
            SpawnEffectEvent::AtEntity(at_entity, spawn_effect_data) => {
                if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data)
                {
                    if let Ok(at_global_transform) = query_transform.get(*at_entity) {
                        if let Some(effect_entity) = spawn_effect(
                            &vfs_resource.vfs,
                            &mut commands,
                            &asset_server,
                            &mut particle_materials,
                            &mut effect_mesh_materials,
                            effect_file_path,
                            false,
                            None,
                        ) {
                            commands
                                .entity(effect_entity)
                                .insert(Transform::from_translation(
                                    at_global_transform.translation,
                                ));
                        }
                    }
                }
            }
            SpawnEffectEvent::OnEntity(on_entity, spawn_effect_data) => {
                if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data)
                {
                    if let Some(effect_entity) = spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        effect_file_path,
                        false,
                        None,
                    ) {
                        commands.entity(*on_entity).add_child(effect_entity);
                    }
                }
            }
            SpawnEffectEvent::OnDummyBone(skeleton_entity, dummy_bone_id, spawn_effect_data) => {
                let mut dummy_entity = *skeleton_entity;

                if let Ok((skinned_mesh, character_model, npc_model)) =
                    query_skeleton.get(*skeleton_entity)
                {
                    if let Some(dummy_bone_offset) = character_model
                        .map(|character_model| character_model.dummy_bone_offset)
                        .or_else(|| npc_model.map(|npc_model| npc_model.dummy_bone_offset))
                    {
                        if let Some(joint) =
                            skinned_mesh.joints.get(dummy_bone_offset + dummy_bone_id)
                        {
                            dummy_entity = *joint;
                        }
                    }
                }

                if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data)
                {
                    if query_transform.get(dummy_entity).is_ok() {
                        if let Some(effect_entity) = spawn_effect(
                            &vfs_resource.vfs,
                            &mut commands,
                            &asset_server,
                            &mut particle_materials,
                            &mut effect_mesh_materials,
                            effect_file_path,
                            false,
                            None,
                        ) {
                            commands.entity(dummy_entity).add_child(effect_entity);
                        }
                    }
                }
            }
            SpawnEffectEvent::WithTransform(transform, spawn_effect_data) => {
                if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data)
                {
                    if let Some(effect_entity) = spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        effect_file_path,
                        false,
                        None,
                    ) {
                        commands.entity(effect_entity).insert(*transform);
                    }
                }
            }
        }
    }
}
