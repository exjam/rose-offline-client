use bevy::{
    math::{Vec3, Quat},
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, Entity, GlobalTransform, Local,
         Res, ResMut, Transform, Query, With, Camera,
    },
    window::Windows,
};
use bevy_egui::{egui, EguiContext};

use crate::{
    character_model::{spawn_character_model, CharacterModelList},
    render::StaticMeshMaterial,
    resources::{CharacterList, NetworkThread, WorldConnection},
};

#[derive(Default)]
pub struct CharacterListModels {
    models: Vec<(String, Entity)>,
}

#[derive(Default)]
pub struct CharacterSelectUiState {}

pub fn character_select_enter_system(mut commands: Commands, mut windows: ResMut<Windows>, mut query_camera: Query<&mut Transform, With<Camera>>) {
    if let Some(window) = windows.get_primary_mut() {
        window.set_cursor_lock_mode(false);
        window.set_cursor_visibility(true);
    }

    *query_camera.single_mut() = Transform::from_xyz(5200.0, 3.4, -5220.0)
    .looking_at(Vec3::new(5200.0, 3.4, -5200.0), Vec3::Y);
}

const CHARACTER_POSITIONS: [[f32; 3]; 5] = [
    [5205.0, 1.0, -5205.0],
    [5202.70, 1.0, -5206.53],
    [5200.00, 1.0, -5207.07],
    [5197.30, 1.0, -5206.53],
    [5195.00, 1.0, -5205.00],
];

#[allow(clippy::too_many_arguments)]
pub fn character_select_system(
    mut commands: Commands,
    mut character_list_models: Local<CharacterListModels>,
    mut ui_state: Local<CharacterSelectUiState>,
    mut egui_context: ResMut<EguiContext>,
    world_connection: Option<Res<WorldConnection>>,
    character_list: Option<Res<CharacterList>>,
    network_thread: Res<NetworkThread>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    if let Some(character_list) = character_list {
        for character in character_list.characters.iter() {
            if !character_list_models
                .models
                .iter()
                .any(|(name, _)| name == &character.info.name)
            {
                let character_model = spawn_character_model(
                    &mut commands,
                    &asset_server,
                    &mut static_mesh_materials,
                    &character_model_list,
                    &character.info,
                    &character.equipment,
                    None,
                );
                let root_bone = character_model.skeleton.bones[0];
                let index = character_list_models.models.len();
                let character_entity = commands
                    .spawn_bundle((
                        character.info.clone(),
                        character.equipment.clone(),
                        character_model,
                        GlobalTransform::default(),
                        Transform::from_translation(CHARACTER_POSITIONS[index].into())
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                            .with_scale(Vec3::new(1.5, 1.5, 1.5)),
                    ))
                    .add_child(root_bone)
                    .id();
                character_list_models
                    .models
                    .push((character.info.name.clone(), character_entity));
            }
        }
    }

    egui::Window::new("Character Select")
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, 0.0])
        .collapsible(false)
        .title_bar(false)
        .show(egui_context.ctx_mut(), |ui| {
            if ui.button("Play").clicked() {
            }
            if ui.button("Exit").clicked() {
            }
        });
}
