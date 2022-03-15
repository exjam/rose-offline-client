use bevy::prelude::{
    AssetServer, Assets, BuildChildren, Changed, Commands, DespawnRecursiveExt, Entity, Query, Res,
    ResMut,
};

use rose_game_common::components::Npc;

use crate::{
    components::NpcModel,
    npc_model::{spawn_npc_model, NpcModelList},
    render::StaticMeshMaterial,
    VfsResource,
};

pub fn npc_model_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Npc, Option<&mut NpcModel>), Changed<Npc>>,
    asset_server: Res<AssetServer>,
    npc_model_list: Res<NpcModelList>,
    vfs_resource: Res<VfsResource>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    for (entity, npc, mut current_npc_model) in query.iter_mut() {
        if let Some(current_npc_model) = current_npc_model.as_mut() {
            if current_npc_model.npc_id == npc.id {
                // Does not need new model, ignore
                continue;
            }

            // Remove old model, replace with new one below
            commands
                .entity(current_npc_model.skeleton.root)
                .despawn_recursive();
        }

        if let Some(npc_model) = spawn_npc_model(
            &mut commands,
            &npc_model_list,
            &asset_server,
            &mut static_mesh_materials,
            npc.id,
            &vfs_resource.vfs,
            None, // Some((bone_mesh, bone_material)),
        ) {
            let root_bone = npc_model.skeleton.root;
            commands
                .entity(entity)
                .insert(npc_model)
                .add_child(root_bone);
        } else if current_npc_model.is_some() {
            commands.entity(entity).remove::<NpcModel>();
        }
    }
}
