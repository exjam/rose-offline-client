use bevy::{
    hierarchy::BuildChildren,
    prelude::{
        AssetServer, Assets, Commands, EventReader, GlobalTransform, Query, Res, ResMut, Transform,
    },
    render::mesh::skinning::SkinnedMesh,
};
use rose_file_readers::VfsPath;

use crate::{
    components::DummyBoneOffset,
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
    query_skeleton: Query<(&SkinnedMesh, &DummyBoneOffset)>,
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
                        spawn_effect_data.manual_despawn,
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
                            spawn_effect_data.manual_despawn,
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
            SpawnEffectEvent::OnEntity(on_entity, dummy_bone_id, spawn_effect_data) => {
                let mut link_entity = *on_entity;

                if let Some(dummy_bone_id) = dummy_bone_id {
                    if let Ok((skinned_mesh, dummy_bone_offset)) = query_skeleton.get(*on_entity) {
                        if let Some(joint) = skinned_mesh
                            .joints
                            .get(dummy_bone_offset.index + dummy_bone_id)
                        {
                            link_entity = *joint;
                        }
                    }
                }

                if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data)
                {
                    if let Some(effect_entity) = spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        effect_file_path,
                        spawn_effect_data.manual_despawn,
                        None,
                    ) {
                        commands.entity(link_entity).add_child(effect_entity);
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
                        spawn_effect_data.manual_despawn,
                        None,
                    ) {
                        commands.entity(effect_entity).insert(*transform);
                    }
                }
            }
        }
    }
}
