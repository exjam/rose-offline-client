use bevy::prelude::{AssetServer, Assets, Changed, Commands, Entity, Or, Query, Res, ResMut};

use rose_game_common::components::{CharacterInfo, Equipment};

use crate::{
    character_model::{spawn_character_model, update_character_equipment, CharacterModelList},
    components::{CharacterModel, ModelSkeleton},
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
            Option<&ModelSkeleton>,
        ),
        Or<(Changed<CharacterInfo>, Changed<Equipment>)>,
    >,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    for (entity, character_info, equipment, mut character_model, model_skeleton) in query.iter_mut()
    {
        if let Some(character_model) = character_model.as_mut() {
            update_character_equipment(
                &mut commands,
                entity,
                &asset_server,
                &mut static_mesh_materials,
                &character_model_list,
                character_model,
                model_skeleton.as_ref().unwrap(),
                character_info,
                equipment,
            );
        } else {
            let (character_model, model_skeleton) = spawn_character_model(
                &mut commands,
                entity,
                &asset_server,
                &mut static_mesh_materials,
                &character_model_list,
                character_info,
                equipment,
            );
            commands
                .entity(entity)
                .insert_bundle((character_model, model_skeleton));
        }
    }
}
