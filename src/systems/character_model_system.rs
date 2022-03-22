use bevy::{
    core::Time,
    prelude::{AssetServer, Assets, Changed, Commands, Entity, Or, Query, Res, ResMut},
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};

use rose_game_common::components::{CharacterInfo, Equipment};

use crate::{
    components::{ActiveMotion, CharacterModel},
    model_loader::ModelLoader,
    render::StaticMeshMaterial,
};

#[allow(clippy::type_complexity)]
pub fn character_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &CharacterInfo,
            &Equipment,
            Option<&mut CharacterModel>,
            Option<&SkinnedMesh>,
        ),
        Or<(Changed<CharacterInfo>, Changed<Equipment>)>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    time: Res<Time>,
) {
    for (entity, character_info, equipment, mut character_model, skinned_mesh) in query.iter_mut() {
        if let Some(character_model) = character_model.as_mut() {
            model_loader.update_character_equipment(
                &mut commands,
                &asset_server,
                &mut static_mesh_materials,
                entity,
                character_info,
                equipment,
                character_model,
                skinned_mesh.as_ref().unwrap(),
            );
        } else {
            let (character_model, skinned_mesh) = model_loader.spawn_character_model(
                &mut commands,
                &asset_server,
                &mut static_mesh_materials,
                &mut skinned_mesh_inverse_bindposes_assets,
                entity,
                character_info,
                equipment,
            );
            commands
                .entity(entity)
                .insert_bundle((character_model, skinned_mesh))
                // TODO: Move animation assignment to animation_system
                .insert(ActiveMotion::new(
                    asset_server.load("3DDATA/MOTION/AVATAR/ONEHAND_RUN_M1.ZMO"),
                    time.seconds_since_startup(),
                ));
        }
    }
}
