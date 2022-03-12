use bevy::prelude::{AssetServer, Assets, Changed, Commands, Query, Res, ResMut};

use rose_game_common::components::{CharacterInfo, Equipment};

use crate::{
    character_model::{update_character_equipment, CharacterModelList},
    components::CharacterModel,
    render::StaticMeshMaterial,
};

pub fn character_model_system(
    mut commands: Commands,
    mut query: Query<(&mut CharacterModel, &CharacterInfo, &Equipment), Changed<Equipment>>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    for (mut character_model, character_info, equipment) in query.iter_mut() {
        update_character_equipment(
            &mut commands,
            &asset_server,
            &mut static_mesh_materials,
            &character_model_list,
            &mut character_model,
            character_info,
            equipment,
        );
    }
}
