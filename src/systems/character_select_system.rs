use bevy::{
    core::Time,
    math::{Quat, Vec3},
    prelude::{
        AssetServer, Commands, DespawnRecursiveExt, Entity, EventReader, EventWriter,
        GlobalTransform, Query, Res, ResMut, State, Transform, With,
    },
    render::camera::Camera3d,
    window::Windows,
};
use bevy_egui::{egui, EguiContext};
use rose_data::ZoneId;
use rose_game_common::messages::client::{ClientMessage, SelectCharacter};

use crate::{
    components::ActiveMotion,
    events::{GameConnectionEvent, LoadZoneEvent, ZoneEvent},
    fly_camera::FlyCameraController,
    follow_camera::FollowCameraController,
    resources::{AppState, CharacterList, GameConnection, ServerConfiguration, WorldConnection},
};

enum CharacterSelectState {
    Entering,
    CharacterSelect,
    // TODO: CharacterCreate,
    ConnectingGameServer,
    Leaving,
    Loading,
}

impl Default for CharacterSelectState {
    fn default() -> Self {
        Self::Entering
    }
}

#[derive(Default)]
pub struct CharacterSelect {
    selected_character_index: usize,
    state: CharacterSelectState,
    join_zone_id: Option<ZoneId>,
}

pub struct CharacterSelectModelList {
    models: Vec<(Option<String>, Entity)>,
}

const CHARACTER_POSITIONS: [[f32; 3]; 5] = [
    [5205.0, 1.0, -5205.0],
    [5202.70, 1.0, -5206.53],
    [5200.00, 1.0, -5207.07],
    [5197.30, 1.0, -5206.53],
    [5195.00, 1.0, -5205.00],
];

pub fn character_select_enter_system(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    query_cameras: Query<Entity, With<Camera3d>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    if let Some(window) = windows.get_primary_mut() {
        window.set_cursor_lock_mode(false);
        window.set_cursor_visibility(true);
    }

    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .insert(
                Transform::from_xyz(5200.0, 3.4, -5220.0)
                    .looking_at(Vec3::new(5200.0, 3.4, -5200.0), Vec3::Y),
            )
            .remove::<FlyCameraController>()
            .remove::<FollowCameraController>()
            .insert(
                ActiveMotion::new(
                    asset_server.load("3DDATA/TITLE/CAMERA01_INSELECT01.ZMO"),
                    time.seconds_since_startup(),
                )
                .with_repeat_limit(1),
            );
    }

    // Reset state
    commands.insert_resource(CharacterSelect::default());

    // Spawn entities to use for character list models
    let mut models = Vec::with_capacity(CHARACTER_POSITIONS.len());
    for position in CHARACTER_POSITIONS {
        let entity = commands
            .spawn_bundle((
                GlobalTransform::default(),
                Transform::from_translation(position.into())
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                    .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            ))
            .id();
        models.push((None, entity));
    }
    commands.insert_resource(CharacterSelectModelList { models });
}

pub fn character_select_exit_system(
    mut commands: Commands,
    model_list: Res<CharacterSelectModelList>,
) {
    // Despawn character models
    for (_, entity) in model_list.models.iter() {
        commands.entity(*entity).despawn_recursive();
    }

    commands.remove_resource::<CharacterList>();
    commands.remove_resource::<CharacterSelect>();
    commands.remove_resource::<CharacterSelectModelList>();
}

pub fn character_select_models_system(
    mut commands: Commands,
    mut model_list: ResMut<CharacterSelectModelList>,
    character_list: Option<Res<CharacterList>>,
) {
    // Ensure all character list models are up to date
    if let Some(character_list) = character_list.as_ref() {
        for (index, character) in character_list.characters.iter().enumerate() {
            if model_list.models[index].0.as_ref() != Some(&character.info.name) {
                commands
                    .entity(model_list.models[index].1)
                    .insert_bundle((character.info.clone(), character.equipment.clone()));
                model_list.models[index].0 = Some(character.info.name.clone());
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn character_select_system(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut character_select_state: ResMut<CharacterSelect>,
    mut egui_context: ResMut<EguiContext>,
    mut zone_events: EventReader<ZoneEvent>,
    mut game_connection_events: EventReader<GameConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    query_camera: Query<(Entity, Option<&ActiveMotion>), With<Camera3d>>,
    game_connection: Option<Res<GameConnection>>,
    world_connection: Option<Res<WorldConnection>>,
    character_list: Option<Res<CharacterList>>,
    server_configuration: Res<ServerConfiguration>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    if world_connection.is_none() {
        // Disconnected, return to login
        app_state.set(AppState::GameLogin).ok();
        return;
    }

    match character_select_state.state {
        CharacterSelectState::Entering => {
            let (_, camera_motion) = query_camera.single();
            if camera_motion.is_none() || server_configuration.auto_login {
                character_select_state.state = CharacterSelectState::CharacterSelect;
            }
        }
        CharacterSelectState::CharacterSelect => {
            egui::Window::new("Character Select")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .title_bar(false)
                .show(egui_context.ctx_mut(), |ui| {
                    let mut try_play_character = false;

                    if let Some(character_list) = character_list.as_ref() {
                        for (i, character) in character_list.characters.iter().enumerate() {
                            ui.selectable_value(
                                &mut character_select_state.selected_character_index,
                                i,
                                &character.info.name,
                            );
                        }
                    }

                    if ui.button("Play").clicked() {
                        try_play_character = true;
                    }

                    if server_configuration.auto_login {
                        if let Some(preset_character_name) =
                            server_configuration.preset_character_name.as_ref()
                        {
                            let mut selected_character_index = None;

                            if let Some(character_list) = character_list.as_ref() {
                                for (i, character) in character_list.characters.iter().enumerate() {
                                    if &character.info.name == preset_character_name {
                                        selected_character_index = Some(i);
                                    }
                                }
                            }

                            if let Some(selected_character_index) = selected_character_index {
                                character_select_state.selected_character_index =
                                    selected_character_index;
                                try_play_character = true;
                            }
                        }
                    }

                    if try_play_character {
                        if let Some(connection) = world_connection.as_ref() {
                            if let Some(character_list) = character_list.as_ref() {
                                connection
                                    .client_message_tx
                                    .send(ClientMessage::SelectCharacter(SelectCharacter {
                                        slot: character_select_state.selected_character_index as u8,
                                        name: character_list.characters
                                            [character_select_state.selected_character_index]
                                            .info
                                            .name
                                            .clone(),
                                    }))
                                    .ok();

                                character_select_state.state =
                                    CharacterSelectState::ConnectingGameServer;
                            }
                        }
                    }

                    if ui.button("Logout").clicked() {
                        commands.remove_resource::<WorldConnection>();
                    }
                });
        }
        CharacterSelectState::ConnectingGameServer => {
            egui::Window::new("Connecting...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Connecting to game");
                });

            for event in game_connection_events.iter() {
                if let &GameConnectionEvent::JoiningZone(zone_id) = event {
                    // Start camera animation
                    let (camera_entity, _) = query_camera.single();
                    commands.entity(camera_entity).insert(
                        ActiveMotion::new(
                            asset_server.load("3DDATA/TITLE/CAMERA01_INGAME01.ZMO"),
                            time.seconds_since_startup(),
                        )
                        .with_repeat_limit(1),
                    );

                    character_select_state.state = CharacterSelectState::Leaving;
                    character_select_state.join_zone_id = Some(zone_id);
                }
            }
        }
        CharacterSelectState::Leaving => {
            let (_, camera_motion) = query_camera.single();
            if camera_motion.is_none() || server_configuration.auto_login {
                // Wait until camera motion complete, then load the zone!
                character_select_state.state = CharacterSelectState::Loading;
                load_zone_events.send(LoadZoneEvent::new(
                    character_select_state.join_zone_id.unwrap(),
                ));
            }
        }
        CharacterSelectState::Loading => {
            for event in zone_events.iter() {
                match event {
                    &ZoneEvent::Loaded(_) => {
                        // Tell server we are ready to join the zone, once the server replies
                        // then game_connection_system will transition app state to in game
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::JoinZoneRequest)
                                .ok();
                        }
                    }
                }
            }
        }
    }
}
