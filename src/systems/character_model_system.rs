use bevy::prelude::{
    AssetServer, Assets, BuildChildren, Changed, Commands, Entity, Or, Query, Res, ResMut,
};

use rose_game_common::components::{CharacterInfo, Equipment};

use crate::{
    character_model::{spawn_character_model, update_character_equipment, CharacterModelList},
    components::CharacterModel,
    render::StaticMeshMaterial,
    resources::DebugBoneVisualisation,
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
        ),
        Or<(Changed<CharacterInfo>, Changed<Equipment>)>,
    >,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    debug_bone_visualisation: Option<Res<DebugBoneVisualisation>>,
) {
    for (entity, character_info, equipment, mut character_model) in query.iter_mut() {
        if let Some(character_model) = character_model.as_mut() {
            update_character_equipment(
                &mut commands,
                &asset_server,
                &mut static_mesh_materials,
                &character_model_list,
                character_model,
                character_info,
                equipment,
            );
        } else {
            let character_model = spawn_character_model(
                &mut commands,
                &asset_server,
                &mut static_mesh_materials,
                &character_model_list,
                character_info,
                equipment,
                debug_bone_visualisation
                    .as_ref()
                    .map(|x| (x.mesh.clone(), x.material.clone())),
            );
            let root_bone = character_model.skeleton.root;
            commands
                .entity(entity)
                .insert(character_model)
                .add_child(root_bone);
        }
    }
}
