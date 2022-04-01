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
use rose_game_common::{
    components::{CharacterGender, CharacterInfo, Equipment},
    messages::{
        client::{ClientMessage, CreateCharacter, SelectCharacter},
        server::CreateCharacterError,
    },
};

use crate::{
    components::ActiveMotion,
    events::{GameConnectionEvent, LoadZoneEvent, WorldConnectionEvent, ZoneEvent},
    fly_camera::FlyCameraController,
    follow_camera::FollowCameraController,
    resources::{AppState, CharacterList, GameConnection, ServerConfiguration, WorldConnection},
};

enum CharacterSelectState {
    Entering,
    CharacterSelect,
    CharacterCreate,
    CharacterCreating,
    ConnectingGameServer,
    Leaving,
    Loading,
}

pub struct CharacterSelect {
    selected_character_index: usize,
    state: CharacterSelectState,
    join_zone_id: Option<ZoneId>,
    create_character_entity: Option<Entity>,
    create_character_name: String,
    create_character_gender: CharacterGender,
    create_character_hair_index: usize,
    create_character_face_index: usize,
    create_character_error_message: String,
}

impl Default for CharacterSelect {
    fn default() -> Self {
        Self {
            selected_character_index: 0,
            state: CharacterSelectState::Entering,
            join_zone_id: None,
            create_character_entity: None,
            create_character_name: String::new(),
            create_character_gender: CharacterGender::Male,
            create_character_hair_index: 0,
            create_character_face_index: 0,
            create_character_error_message: String::new(),
        }
    }
}

pub struct CharacterSelectModelList {
    models: Vec<(Option<String>, Entity)>,
}

const CREATE_CHARACTER_FACE_LIST: [u8; 7] = [1, 8, 15, 22, 29, 36, 43];

const CREATE_CHARACTER_HAIR_LIST: [u8; 5] = [0, 5, 10, 15, 20];

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
            .insert(ActiveMotion::new_once(
                asset_server.load("3DDATA/TITLE/CAMERA01_INSELECT01.ZMO"),
                time.seconds_since_startup(),
            ));
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
    character_select_state: Res<CharacterSelect>,
    model_list: Res<CharacterSelectModelList>,
) {
    // Despawn character models
    for (_, entity) in model_list.models.iter() {
        commands.entity(*entity).despawn_recursive();
    }

    if let Some(entity) = character_select_state.create_character_entity {
        commands.entity(entity).despawn_recursive();
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
    mut world_connection_events: EventReader<WorldConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    query_camera: Query<(Entity, Option<&ActiveMotion>), With<Camera3d>>,
    mut query_create_character_info: Query<&mut CharacterInfo>,
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

                    if ui.button("Create").clicked() {
                        let (camera_entity, _) = query_camera.single();
                        commands
                            .entity(camera_entity)
                            .insert(ActiveMotion::new_once(
                                asset_server.load("3DDATA/TITLE/CAMERA01_CREATE01.ZMO"),
                                time.seconds_since_startup(),
                            ));

                        character_select_state.state = CharacterSelectState::CharacterCreate;
                    }

                    if ui.button("Logout").clicked() {
                        commands.remove_resource::<WorldConnection>();
                    }
                });
        }
        CharacterSelectState::CharacterCreate => {
            egui::Window::new("Character Create")
                .anchor(egui::Align2::CENTER_CENTER, [-200.0, 0.0])
                .collapsible(false)
                .title_bar(false)
                .show(egui_context.ctx_mut(), |ui| {
                    egui::Grid::new("character_create_grid")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(
                                &mut character_select_state.create_character_name,
                            );
                            ui.end_row();

                            ui.label("Gender");
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut character_select_state.create_character_gender,
                                    CharacterGender::Male,
                                    "Male",
                                );
                                ui.radio_value(
                                    &mut character_select_state.create_character_gender,
                                    CharacterGender::Female,
                                    "Female",
                                );
                            });
                            ui.end_row();

                            ui.label("Face");
                            ui.horizontal(|ui| {
                                ui.add_enabled_ui(
                                    character_select_state.create_character_face_index > 0,
                                    |ui| {
                                        if ui.button("⬅").clicked() {
                                            character_select_state.create_character_face_index -= 1;
                                        }
                                    },
                                );
                                ui.label(format!(
                                    "{}",
                                    character_select_state.create_character_face_index + 1
                                ));
                                ui.add_enabled_ui(
                                    character_select_state.create_character_face_index + 1
                                        < CREATE_CHARACTER_FACE_LIST.len(),
                                    |ui| {
                                        if ui.button("➡").clicked() {
                                            character_select_state.create_character_face_index += 1;
                                        }
                                    },
                                );
                            });
                            ui.end_row();

                            ui.label("Hair");
                            ui.horizontal(|ui| {
                                ui.add_enabled_ui(
                                    character_select_state.create_character_hair_index > 0,
                                    |ui| {
                                        if ui.button("<").clicked() {
                                            character_select_state.create_character_hair_index -= 1;
                                        }
                                    },
                                );
                                ui.label(format!(
                                    "{}",
                                    character_select_state.create_character_hair_index + 1
                                ));
                                ui.add_enabled_ui(
                                    character_select_state.create_character_hair_index + 1
                                        < CREATE_CHARACTER_HAIR_LIST.len(),
                                    |ui| {
                                        if ui.button(">").clicked() {
                                            character_select_state.create_character_hair_index += 1;
                                        }
                                    },
                                );
                            });
                            ui.end_row();
                        });

                    ui.add_enabled_ui(
                        character_select_state.create_character_name.len() > 3,
                        |ui| {
                            if ui.button("Create").clicked() {
                                if let Some(world_connection) = world_connection.as_ref() {
                                    world_connection
                                        .client_message_tx
                                        .send(ClientMessage::CreateCharacter(CreateCharacter {
                                            gender: character_select_state.create_character_gender,
                                            birth_stone: 0,
                                            hair: CREATE_CHARACTER_HAIR_LIST
                                                [character_select_state
                                                    .create_character_hair_index],
                                            face: CREATE_CHARACTER_FACE_LIST
                                                [character_select_state
                                                    .create_character_face_index],
                                            name: character_select_state
                                                .create_character_name
                                                .clone(),
                                        }))
                                        .ok();
                                }
                                character_select_state.state =
                                    CharacterSelectState::CharacterCreating;
                            }
                        },
                    );

                    if ui.button("Cancel").clicked() {
                        let (camera_entity, _) = query_camera.single();
                        commands
                            .entity(camera_entity)
                            .insert(ActiveMotion::new_once(
                                asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
                                time.seconds_since_startup(),
                            ));
                        character_select_state.state = CharacterSelectState::CharacterSelect;
                    }

                    if !character_select_state
                        .create_character_error_message
                        .is_empty()
                    {
                        ui.label(
                            egui::RichText::new(format!(
                                "Error: {}",
                                character_select_state.create_character_error_message
                            ))
                            .color(egui::Color32::RED),
                        );
                    }
                });

            if let Some(create_character_entity) = character_select_state.create_character_entity {
                if let Ok(mut create_character_info) =
                    query_create_character_info.get_mut(create_character_entity)
                {
                    let face = CREATE_CHARACTER_FACE_LIST
                        [character_select_state.create_character_face_index];
                    let hair = CREATE_CHARACTER_HAIR_LIST
                        [character_select_state.create_character_hair_index];

                    if create_character_info.face != face {
                        create_character_info.face = face;
                    }

                    if create_character_info.hair != hair {
                        create_character_info.hair = hair;
                    }

                    if create_character_info.gender
                        != character_select_state.create_character_gender
                    {
                        create_character_info.gender =
                            character_select_state.create_character_gender;
                    }
                }
            } else {
                let create_character_entity = commands
                    .spawn_bundle((
                        CharacterInfo {
                            name: "CreateCharacter".to_string(),
                            gender: character_select_state.create_character_gender,
                            race: 0,
                            birth_stone: 0,
                            job: 0,
                            face: CREATE_CHARACTER_FACE_LIST
                                [character_select_state.create_character_face_index],
                            hair: CREATE_CHARACTER_HAIR_LIST
                                [character_select_state.create_character_hair_index],
                            rank: 0,
                            fame: 0,
                            fame_b: 0,
                            fame_g: 0,
                            revive_zone_id: ZoneId::new(1).unwrap(),
                            revive_position: Vec3::new(5200.0, 5200.0, 0.0),
                            unique_id: 0,
                        },
                        Equipment::new(),
                        Transform::from_translation(Vec3::new(5200.05, 7.47, -5200.18))
                            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                            .with_scale(Vec3::new(1.5, 1.5, 1.5)),
                        GlobalTransform::default(),
                    ))
                    .id();
                character_select_state.create_character_entity = Some(create_character_entity);
            }
        }
        CharacterSelectState::CharacterCreating => {
            egui::Window::new("Creating character...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Creating character");
                });

            // Process response from server
            for event in world_connection_events.iter() {
                let WorldConnectionEvent::CreateCharacterResponse(response) = event;
                match response {
                    Ok(_) => {
                        let (camera_entity, _) = query_camera.single();
                        commands
                            .entity(camera_entity)
                            .insert(ActiveMotion::new_once(
                                asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
                                time.seconds_since_startup(),
                            ));
                        character_select_state.state = CharacterSelectState::CharacterSelect;

                        if let Some(world_connection) = world_connection.as_ref() {
                            world_connection
                                .client_message_tx
                                .send(ClientMessage::GetCharacterList)
                                .ok();
                        }
                    }
                    Err(CreateCharacterError::Failed) => {
                        character_select_state.create_character_error_message =
                            "Unknown error creating character".into();
                        character_select_state.state = CharacterSelectState::CharacterCreate;
                    }
                    Err(CreateCharacterError::AlreadyExists) => {
                        character_select_state.create_character_error_message =
                            "Character name already exists".into();
                        character_select_state.state = CharacterSelectState::CharacterCreate;
                    }
                    Err(CreateCharacterError::NoMoreSlots) => {
                        character_select_state.create_character_error_message =
                            "Cannot create more characters".into();
                        character_select_state.state = CharacterSelectState::CharacterCreate;
                    }
                    Err(CreateCharacterError::InvalidValue) => {
                        character_select_state.create_character_error_message =
                            "Invalid value".into();
                        character_select_state.state = CharacterSelectState::CharacterCreate;
                    }
                }
            }
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
                    commands
                        .entity(camera_entity)
                        .insert(ActiveMotion::new_once(
                            asset_server.load("3DDATA/TITLE/CAMERA01_INGAME01.ZMO"),
                            time.seconds_since_startup(),
                        ));

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

    if !matches!(
        character_select_state.state,
        CharacterSelectState::CharacterCreate | CharacterSelectState::CharacterCreating
    ) {
        if let Some(entity) = character_select_state.create_character_entity.take() {
            commands.entity(entity).despawn_recursive();
        }
        character_select_state
            .create_character_error_message
            .clear();
    }
}
