use bevy::{
    core::Time,
    math::Vec3,
    prelude::{AssetServer, Assets, Changed, Commands, Entity, Query, Res, ResMut, Transform},
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};

use rose_game_common::components::Npc;

use crate::{
    components::{ActiveMotion, NpcModel},
    model_loader::ModelLoader,
    render::StaticMeshMaterial,
    resources::GameData,
};

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn npc_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Npc,
            Option<&mut NpcModel>,
            Option<&SkinnedMesh>,
            &Transform,
        ),
        Changed<Npc>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    for (entity, npc, mut current_npc_model, skinned_mesh, transform) in query.iter_mut() {
        if let Some(current_npc_model) = current_npc_model.as_mut() {
            if current_npc_model.npc_id == npc.id {
                // Does not need new model, ignore
                continue;
            }

            // Despawn model parts
            for part_entity in current_npc_model.model_parts.iter() {
                commands.entity(*part_entity).despawn();
            }

            // Despawn model skeleton
            if let Some(skinned_mesh) = skinned_mesh {
                for bone_entity in skinned_mesh.joints.iter() {
                    commands.entity(*bone_entity).despawn();
                }
            }
        }

        if let Some((npc_model, skinned_mesh)) = model_loader.spawn_npc_model(
            &mut commands,
            &asset_server,
            &mut static_mesh_materials,
            &mut skinned_mesh_inverse_bindposes_assets,
            entity,
            npc.id,
        ) {
            // TODO: Move animation assignment to animation_system
            let motion = npc_model
                .action_motions
                .iter()
                .find(|(action_id, _)| *action_id == 1)
                .or_else(|| npc_model.action_motions.get(0));
            if let Some((_, motion)) = motion {
                commands.entity(entity).insert(ActiveMotion::new(
                    motion.clone(),
                    time.seconds_since_startup(),
                ));
            }

            let transform = if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                transform.with_scale(Vec3::new(npc_data.scale, npc_data.scale, npc_data.scale))
            } else {
                *transform
            };

            commands
                .entity(entity)
                .insert_bundle((npc_model, skinned_mesh, transform));
        } else {
            commands
                .entity(entity)
                .insert(NpcModel {
                    npc_id: npc.id,
                    model_parts: Vec::new(),
                    dummy_bone_offset: 0,
                    action_motions: Vec::new(),
                })
                .remove::<SkinnedMesh>();
        }
    }
}
