use bevy::prelude::{AssetServer, Assets, Changed, Commands, Entity, Query, Res, ResMut};

use rose_game_common::components::Npc;

use crate::{
    components::{ModelSkeleton, NpcModel},
    npc_model::{spawn_npc_model, NpcModelList},
    render::StaticMeshMaterial,
    resources::DebugBoneVisualisation,
    VfsResource,
};

#[allow(clippy::type_complexity)]
pub fn npc_model_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Npc, Option<&mut NpcModel>, Option<&ModelSkeleton>), Changed<Npc>>,
    asset_server: Res<AssetServer>,
    npc_model_list: Res<NpcModelList>,
    vfs_resource: Res<VfsResource>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    debug_bone_visualisation: Option<Res<DebugBoneVisualisation>>,
) {
    for (entity, npc, mut current_npc_model, current_skeleton) in query.iter_mut() {
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
            if let Some(current_skeleton) = current_skeleton {
                for bone_entity in current_skeleton.bones.iter() {
                    commands.entity(*bone_entity).despawn();
                }
            }
        }

        if let Some((npc_model, model_skeleton)) = spawn_npc_model(
            &mut commands,
            entity,
            &npc_model_list,
            &asset_server,
            &mut static_mesh_materials,
            npc.id,
            &vfs_resource.vfs,
            debug_bone_visualisation
                .as_ref()
                .map(|x| (x.mesh.clone(), x.material.clone())),
        ) {
            commands
                .entity(entity)
                .insert_bundle((npc_model, model_skeleton));
        } else {
            if current_npc_model.is_some() {
                commands.entity(entity).remove::<NpcModel>();
            }

            if current_skeleton.is_some() {
                commands.entity(entity).remove::<ModelSkeleton>();
            }
        }
    }
}
