use bevy::{
    math::Vec3,
    prelude::{
        AssetServer, Assets, Changed, Commands, DespawnRecursiveExt, Entity, Query, Res, ResMut,
        Transform,
    },
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};
use enum_map::EnumMap;

use rose_game_common::components::Npc;

use crate::{
    components::{ClientEntityName, DummyBoneOffset, ModelHeight, NpcModel, RemoveColliderCommand},
    model_loader::ModelLoader,
    render::{EffectMeshMaterial, ObjectMaterial, ParticleMaterial},
    resources::GameData,
};

pub fn npc_model_update_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Npc,
            &Transform,
            Option<&mut NpcModel>,
            Option<&mut SkinnedMesh>,
            Option<&mut DummyBoneOffset>,
        ),
        Changed<Npc>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    game_data: Res<GameData>,
) {
    for (
        entity,
        npc,
        transform,
        mut current_npc_model,
        mut current_skinned_mesh,
        current_dummy_bone_offset,
    ) in query.iter_mut()
    {
        if let Some(previous_npc_model) = current_npc_model.as_mut() {
            if npc.id == previous_npc_model.npc_id {
                // NPC model has not changed
                continue;
            }

            // Despawn model parts
            for part_entity in previous_npc_model.model_parts.drain(..) {
                commands.entity(part_entity).despawn_recursive();
            }

            // Despawn model skeleton
            if let Some(current_skinned_mesh) = current_skinned_mesh.as_mut() {
                for bone_entity in current_skinned_mesh.joints.drain(..) {
                    commands.entity(bone_entity).despawn_recursive();
                }
            }

            // Remove the old model collider and height
            commands
                .entity(entity)
                .remove_and_despawn_collider()
                .remove::<ModelHeight>();
        }

        let (npc_model, skinned_mesh, dummy_bone_offset) =
            if let Some((npc_model, skinned_mesh, dummy_bone_offset)) = model_loader
                .spawn_npc_model(
                    &mut commands,
                    &asset_server,
                    &mut effect_mesh_materials,
                    &mut particle_materials,
                    &mut object_materials,
                    &mut skinned_mesh_inverse_bindposes_assets,
                    entity,
                    npc.id,
                )
            {
                (npc_model, skinned_mesh, dummy_bone_offset)
            } else {
                // Insert empty model so we do not retry every frame.
                (
                    NpcModel {
                        npc_id: npc.id,
                        model_parts: Vec::new(),
                        action_motions: EnumMap::default(),
                        root_bone_position: Vec3::ZERO,
                    },
                    SkinnedMesh::default(),
                    DummyBoneOffset { index: 0 },
                )
            };

        let mut entity_commands = commands.entity(entity);

        // Update scale
        if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
            entity_commands.insert(transform.with_scale(Vec3::new(
                npc_data.scale,
                npc_data.scale,
                npc_data.scale,
            )));
        }

        // Update ClientEntityName
        entity_commands.insert(ClientEntityName::new(
            game_data
                .npcs
                .get_npc(npc.id)
                .map(|npc_data| npc_data.name.to_string())
                .unwrap_or_else(|| format!("??? [{}]", npc.id.get())),
        ));

        // Update model
        if let Some(mut current_npc_model) = current_npc_model {
            *current_npc_model = npc_model;
        } else {
            entity_commands.insert(npc_model);
        }

        if let Some(mut current_skinned_mesh) = current_skinned_mesh {
            *current_skinned_mesh = skinned_mesh;
        } else {
            entity_commands.insert(skinned_mesh);
        }

        if let Some(mut current_dummy_bone_offset) = current_dummy_bone_offset {
            *current_dummy_bone_offset = dummy_bone_offset;
        } else {
            entity_commands.insert(dummy_bone_offset);
        }
    }
}
