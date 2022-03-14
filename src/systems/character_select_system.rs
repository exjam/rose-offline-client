use bevy::{
    math::{Quat, Vec3},
    prelude::{
        AssetServer, Assets, BuildChildren, Camera, Commands, DespawnRecursiveExt, Entity,
        GlobalTransform, Query, Res, ResMut, State, Transform, With,
    },
    window::Windows,
};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::{ClientMessage, SelectCharacter};

use crate::{
    character_model::{spawn_character_model, CharacterModelList},
    render::StaticMeshMaterial,
    resources::{AppState, CharacterList, WorldConnection},
};

enum CharacterSelectState {
    CharacterSelect,
    // TODO: CharacterCreate,
    JoinGameServer,
}

impl Default for CharacterSelectState {
    fn default() -> Self {
        Self::CharacterSelect
    }
}

#[derive(Default)]
pub struct CharacterSelect {
    selected_character_index: usize,
    state: CharacterSelectState,
    models: Vec<(String, Entity)>,
}

pub fn character_select_enter_system(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    mut query_camera: Query<&mut Transform, With<Camera>>,
) {
    if let Some(window) = windows.get_primary_mut() {
        window.set_cursor_lock_mode(false);
        window.set_cursor_visibility(true);
    }

    *query_camera.single_mut() = Transform::from_xyz(5200.0, 3.4, -5220.0)
        .looking_at(Vec3::new(5200.0, 3.4, -5200.0), Vec3::Y);
    commands.insert_resource(CharacterSelect::default());
}

pub fn character_select_exit_system(mut commands: Commands, ui_state: Res<CharacterSelect>) {
    // Despawn character models
    for (_, entity) in ui_state.models.iter() {
        commands.entity(*entity).despawn_recursive();
    }

    commands.remove_resource::<CharacterList>();
    commands.remove_resource::<CharacterSelect>();
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
    mut ui_state: ResMut<CharacterSelect>,
    mut egui_context: ResMut<EguiContext>,
    world_connection: Option<Res<WorldConnection>>,
    character_list: Option<Res<CharacterList>>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut app_state: ResMut<State<AppState>>,
) {
    if world_connection.is_none() {
        // Disconnected, return to login
        app_state.set(AppState::GameLogin).ok();
        return;
    }

    if let Some(character_list) = character_list.as_ref() {
        for character in character_list.characters.iter() {
            if !ui_state
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
                let index = ui_state.models.len();
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
                ui_state
                    .models
                    .push((character.info.name.clone(), character_entity));
            }
        }
    }

    egui::Window::new("Character Select")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .title_bar(false)
        .show(egui_context.ctx_mut(), |ui| {
            if let Some(character_list) = character_list.as_ref() {
                for (i, character) in character_list.characters.iter().enumerate() {
                    ui.selectable_value(
                        &mut ui_state.selected_character_index,
                        i,
                        &character.info.name,
                    );
                }
            }

            if ui.button("Play").clicked() {
                if let Some(connection) = world_connection.as_ref() {
                    if let Some(character_list) = character_list.as_ref() {
                        connection
                            .client_message_tx
                            .send(ClientMessage::SelectCharacter(SelectCharacter {
                                slot: ui_state.selected_character_index as u8,
                                name: character_list.characters[ui_state.selected_character_index]
                                    .info
                                    .name
                                    .clone(),
                            }))
                            .ok();

                        ui_state.state = CharacterSelectState::JoinGameServer;
                    }
                }
            }

            if ui.button("Logout").clicked() {
                commands.remove_resource::<WorldConnection>();
            }
        });
}
