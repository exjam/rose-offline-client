use std::time::{Duration, Instant};

use bevy::{
    input::Input,
    math::{Quat, Vec3},
    prelude::{
        AssetServer, Assets, Camera, Camera3d, Commands, Component, DespawnRecursiveExt, Entity,
        EventReader, EventWriter, GlobalTransform, MouseButton, Query, Res, ResMut, State,
        Transform, With,
    },
    render::{camera::Projection, mesh::skinning::SkinnedMesh},
    window::Windows,
};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::prelude::{InteractionGroups, RapierContext};
use rose_data::{CharacterMotionAction, ZoneId};
use rose_game_common::{
    components::{CharacterGender, CharacterInfo, Equipment},
    messages::{
        client::{ClientMessage, CreateCharacter, DeleteCharacter, SelectCharacter},
        server::CreateCharacterError,
    },
};

use crate::{
    components::{ActiveMotion, CharacterModel, ColliderParent, COLLISION_FILTER_CLICKABLE},
    events::{GameConnectionEvent, LoadZoneEvent, WorldConnectionEvent},
    free_camera::FreeCamera,
    orbit_camera::OrbitCamera,
    ray_from_screenspace::ray_from_screenspace,
    resources::{AppState, CharacterList, ServerConfiguration, UiResources, WorldConnection},
    ui::{
        widgets::{DataBindings, Dialog, DrawText, Widget},
        DialogInstance,
    },
};

#[derive(Copy, Clone, PartialEq)]
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
    selected_character_index: Option<usize>,
    last_selected_time: Option<Instant>,
    try_play_character: bool,
    state: CharacterSelectState,
    prev_state: CharacterSelectState,
    join_zone_id: Option<ZoneId>,
    create_character_entity: Option<Entity>,
    create_character_name: String,
    create_character_gender: CharacterGender,
    create_character_hair_index: usize,
    create_character_face_index: usize,
    create_character_startpos_index: usize,
    create_character_birthstone_index: usize,
    create_character_error_message: String,
    select_dialog_instance: DialogInstance,
}

impl Default for CharacterSelect {
    fn default() -> Self {
        Self {
            selected_character_index: None,
            last_selected_time: None,
            try_play_character: false,
            state: CharacterSelectState::Entering,
            prev_state: CharacterSelectState::Entering,
            join_zone_id: None,
            create_character_entity: None,
            create_character_name: String::new(),
            create_character_gender: CharacterGender::Male,
            create_character_hair_index: 0,
            create_character_face_index: 0,
            create_character_startpos_index: 0,
            create_character_birthstone_index: 0,
            create_character_error_message: String::new(),
            select_dialog_instance: DialogInstance::new("DLGSELAVATAR.XML"),
        }
    }
}

#[derive(Component)]
pub struct CharacterSelectCharacter {
    pub index: usize,
}

pub struct CharacterSelectModelList {
    models: Vec<(Option<String>, Entity)>,
}

const CREATE_CHARACTER_FACE_LIST: [i32; 7] = [1, 8, 15, 22, 29, 36, 43];
const CREATE_CHARACTER_HAIR_LIST: [i32; 5] = [0, 5, 10, 15, 20];
const CREATE_CHARACTER_STARTPOS_LIST: [&str; 5] =
    ["Brave", "Wisdom", "Faith", "Justice", "Liberty"];
const CREATE_CHARACTER_BIRTHSTONE_LIST: [&str; 12] = [
    "Garnet",
    "Amethyst",
    "Aquamarine",
    "Diamond",
    "Emerald",
    "Pearl",
    "Rubi",
    "Peridot",
    "Sapphire",
    "Opal",
    "Topaz",
    "Turquoise",
];

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
            .remove::<FreeCamera>()
            .remove::<OrbitCamera>()
            .insert(ActiveMotion::new_once(
                asset_server.load("3DDATA/TITLE/CAMERA01_INSELECT01.ZMO"),
            ));
    }

    // Reset state
    commands.insert_resource(CharacterSelect::default());

    // Spawn entities to use for character list models
    let mut models = Vec::with_capacity(CHARACTER_POSITIONS.len());
    for (index, position) in CHARACTER_POSITIONS.into_iter().enumerate() {
        let entity = commands
            .spawn_bundle((
                CharacterSelectCharacter { index },
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
    query_characters: Query<(Option<&ActiveMotion>, &CharacterModel), With<SkinnedMesh>>,
) {
    // Ensure all character list models are up to date
    if let Some(character_list) = character_list.as_ref() {
        for (index, character) in character_list.characters.iter().enumerate() {
            let entity = model_list.models[index].1;

            // If the character list has changed, recreate model
            if model_list.models[index].0.as_ref() != Some(&character.info.name) {
                commands
                    .entity(model_list.models[index].1)
                    .insert_bundle((character.info.clone(), character.equipment.clone()));
                model_list.models[index].0 = Some(character.info.name.clone());
            }

            if character.delete_time.is_some() {}

            if let Ok((active_motion, character_model)) = query_characters.get(entity) {
                let desired_motion = if character.delete_time.is_some() {
                    &character_model.action_motions[CharacterMotionAction::Sit]
                } else {
                    &character_model.action_motions[CharacterMotionAction::Stop1]
                };

                if active_motion.map_or(true, |x| x.motion.id != desired_motion.id) {
                    commands
                        .entity(entity)
                        .insert(ActiveMotion::new_repeating(desired_motion.clone()));
                }
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
    mut game_connection_events: EventReader<GameConnectionEvent>,
    mut world_connection_events: EventReader<WorldConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    query_camera: Query<(Entity, &Camera, &GlobalTransform, Option<&ActiveMotion>), With<Camera3d>>,
    mut query_create_character_info: Query<&mut CharacterInfo>,
    world_connection: Option<Res<WorldConnection>>,
    mut character_list: Option<ResMut<CharacterList>>,
    (server_configuration, asset_server): (Res<ServerConfiguration>, Res<AssetServer>),
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let character_select_state = &mut *character_select_state;
    if world_connection.is_none() {
        // Disconnected, return to login
        app_state.set(AppState::GameLogin).ok();
        return;
    }

    for event in world_connection_events.iter() {
        match event {
            WorldConnectionEvent::CreateCharacterResponse(response) => match response {
                Ok(_) => {
                    let (camera_entity, _, _, _) = query_camera.single();
                    commands
                        .entity(camera_entity)
                        .insert(ActiveMotion::new_once(
                            asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
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
                    character_select_state.create_character_error_message = "Invalid value".into();
                    character_select_state.state = CharacterSelectState::CharacterCreate;
                }
            },
            WorldConnectionEvent::DeleteCharacterResponse(response) => match response {
                Ok(response) => {
                    if let Some(character_list) = character_list.as_mut() {
                        for character in character_list.characters.iter_mut() {
                            if character.info.name == response.name {
                                character.delete_time = response.delete_time.clone();
                            }
                        }
                    } else if let Some(world_connection) = world_connection.as_ref() {
                        world_connection
                            .client_message_tx
                            .send(ClientMessage::GetCharacterList)
                            .ok();
                    }
                }
                Err(_) => {
                    // TODO: Show delete character error message
                }
            },
        }
    }

    match character_select_state.state {
        CharacterSelectState::Entering => {
            let (_, _, _, camera_motion) = query_camera.single();
            if camera_motion.is_none() || server_configuration.auto_login {
                character_select_state.state = CharacterSelectState::CharacterSelect;
            }
        }
        CharacterSelectState::CharacterSelect => {
            const IID_BTN_CREATE: i32 = 10;
            const IID_BTN_DELETE: i32 = 11;
            const IID_BTN_OK: i32 = 12;
            const IID_BTN_CANCEL: i32 = 13;

            let dialog = if let Some(dialog) = character_select_state
                .select_dialog_instance
                .get_mut(&dialog_assets, &ui_resources)
            {
                dialog
            } else {
                return;
            };

            let screen_size = egui_context.ctx_mut().input().screen_rect().size();

            if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_CANCEL) {
                button.x = screen_size.x / 5.0 - button.width / 2.0;
                button.y = screen_size.y - button.height;
            }

            if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_CREATE) {
                button.x = screen_size.x * 2.0 / 5.0 - button.width / 2.0;
                button.y = screen_size.y - button.height;
            }

            if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_DELETE) {
                button.x = screen_size.x * 3.0 / 5.0 - button.width / 2.0;
                button.y = screen_size.y - button.height;
            }

            if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_OK) {
                button.x = screen_size.x * 4.0 / 5.0 - button.width / 2.0;
                button.y = screen_size.y - button.height;
            }

            let mut response_create_button = None;
            let mut response_delete_button = None;
            let mut response_ok_button = None;
            let mut response_cancel_button = None;

            egui::Window::new("Character Select")
                .anchor(egui::Align2::LEFT_BOTTOM, [0.0, -24.0 - 40.0])
                .frame(egui::Frame::none())
                .title_bar(false)
                .resizable(false)
                .default_width(screen_size.x)
                .default_height(40.0)
                .show(egui_context.ctx_mut(), |ui| {
                    dialog.draw(
                        ui,
                        DataBindings {
                            response: &mut [
                                (IID_BTN_CREATE, &mut response_create_button),
                                (IID_BTN_DELETE, &mut response_delete_button),
                                (IID_BTN_OK, &mut response_ok_button),
                                (IID_BTN_CANCEL, &mut response_cancel_button),
                            ],
                            ..Default::default()
                        },
                        |_, _| {},
                    );
                });

            let mut try_play_character = character_select_state.try_play_character;
            character_select_state.try_play_character = false;

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
                            Some(selected_character_index);
                        try_play_character = true;
                    }
                }
            }

            if response_create_button.map_or(false, |r| r.clicked())
                && character_list.as_ref().map_or(true, |character_list| {
                    character_list.characters.len() < CHARACTER_POSITIONS.len()
                })
            {
                let (camera_entity, _, _, _) = query_camera.single();
                commands
                    .entity(camera_entity)
                    .insert(ActiveMotion::new_once(
                        asset_server.load("3DDATA/TITLE/CAMERA01_CREATE01.ZMO"),
                    ));

                character_select_state.state = CharacterSelectState::CharacterCreate;
            }

            if response_delete_button.map_or(false, |r| r.clicked()) {
                if let Some(selected_character_index) =
                    character_select_state.selected_character_index
                {
                    if let Some(connection) = world_connection.as_ref() {
                        if let Some(character_list) = character_list.as_ref() {
                            if let Some(selected_character) =
                                character_list.characters.get(selected_character_index)
                            {
                                connection
                                    .client_message_tx
                                    .send(ClientMessage::DeleteCharacter(DeleteCharacter {
                                        slot: selected_character_index as u8,
                                        name: selected_character.info.name.clone(),
                                        is_delete: selected_character.delete_time.is_none(),
                                    }))
                                    .ok();
                            }
                        }
                    }
                }
            }

            if response_ok_button.map_or(false, |r| r.clicked())
                && character_select_state.selected_character_index.is_some()
            {
                try_play_character = true;
            }

            if response_cancel_button.map_or(false, |r| r.clicked()) {
                commands.remove_resource::<WorldConnection>();
            }

            if try_play_character {
                if let Some(selected_character_index) =
                    character_select_state.selected_character_index
                {
                    if let Some(connection) = world_connection.as_ref() {
                        if let Some(character_list) = character_list.as_ref() {
                            if let Some(selected_character) =
                                character_list.characters.get(selected_character_index)
                            {
                                if selected_character.delete_time.is_none() {
                                    connection
                                        .client_message_tx
                                        .send(ClientMessage::SelectCharacter(SelectCharacter {
                                            slot: selected_character_index as u8,
                                            name: selected_character.info.name.clone(),
                                        }))
                                        .ok();

                                    character_select_state.state =
                                        CharacterSelectState::ConnectingGameServer;
                                }
                            }
                        }
                    }
                }
            }
        }
        CharacterSelectState::CharacterCreate => {
            const IID_EDITBOX: i32 = 7;
            const IID_BTN_OK: i32 = 10;
            const IID_BTN_CANCEL: i32 = 11;
            const IID_BTN_LEFT_SEX: i32 = 20;
            const IID_BTN_LEFT_FACE: i32 = 21;
            const IID_BTN_LEFT_HAIR: i32 = 22;
            const IID_BTN_LEFT_STARTPOS: i32 = 23;
            const IID_BTN_LEFT_BIRTHSTONE: i32 = 24;
            const IID_BTN_RIGHT_SEX: i32 = 30;
            const IID_BTN_RIGHT_FACE: i32 = 31;
            const IID_BTN_RIGHT_HAIR: i32 = 32;
            const IID_BTN_RIGHT_STARTPOS: i32 = 33;
            const IID_BTN_RIGHT_BIRTHSTONE: i32 = 34;

            let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_create_avatar)
            {
                dialog
            } else {
                return;
            };

            let mut response_ok = None;
            let mut response_cancel = None;
            let mut response_editbox = None;
            let mut response_prev_sex = None;
            let mut response_prev_face = None;
            let mut response_prev_hair = None;
            let mut response_prev_startpos = None;
            let mut response_prev_birthstone = None;
            let mut response_next_sex = None;
            let mut response_next_face = None;
            let mut response_next_hair = None;
            let mut response_next_startpos = None;
            let mut response_next_birthstone = None;

            let screen_size = egui_context.ctx_mut().input().screen_rect().max;

            egui::Window::new("Character Create")
                .frame(egui::Frame::none())
                .title_bar(false)
                .resizable(false)
                .fixed_pos(egui::pos2(
                    screen_size.x / 4.0 - dialog.width / 2.0,
                    screen_size.y / 2.0 - dialog.height / 2.0,
                ))
                .show(egui_context.ctx_mut(), |ui| {
                    dialog.draw(
                        ui,
                        DataBindings {
                            text: &mut [(
                                IID_EDITBOX,
                                &mut character_select_state.create_character_name,
                            )],
                            response: &mut [
                                (IID_BTN_OK, &mut response_ok),
                                (IID_BTN_CANCEL, &mut response_cancel),
                                (IID_EDITBOX, &mut response_editbox),
                                (IID_BTN_LEFT_SEX, &mut response_prev_sex),
                                (IID_BTN_LEFT_FACE, &mut response_prev_face),
                                (IID_BTN_LEFT_HAIR, &mut response_prev_hair),
                                (IID_BTN_LEFT_STARTPOS, &mut response_prev_startpos),
                                (IID_BTN_LEFT_BIRTHSTONE, &mut response_prev_birthstone),
                                (IID_BTN_RIGHT_SEX, &mut response_next_sex),
                                (IID_BTN_RIGHT_FACE, &mut response_next_face),
                                (IID_BTN_RIGHT_HAIR, &mut response_next_hair),
                                (IID_BTN_RIGHT_STARTPOS, &mut response_next_startpos),
                                (IID_BTN_RIGHT_BIRTHSTONE, &mut response_next_birthstone),
                            ],
                            ..Default::default()
                        },
                        |ui, _| {
                            ui.add_label_in(
                                egui::Rect::from_min_size(
                                    egui::pos2(172.0, 155.0),
                                    egui::vec2(63.0, 19.0),
                                ),
                                match character_select_state.create_character_gender {
                                    CharacterGender::Male => "Male",
                                    CharacterGender::Female => "Female",
                                },
                            );
                            ui.add_label_in(
                                egui::Rect::from_min_size(
                                    egui::pos2(172.0, 185.0),
                                    egui::vec2(63.0, 19.0),
                                ),
                                &format!(
                                    "{}",
                                    character_select_state.create_character_face_index + 1
                                ),
                            );
                            ui.add_label_in(
                                egui::Rect::from_min_size(
                                    egui::pos2(172.0, 215.0),
                                    egui::vec2(63.0, 19.0),
                                ),
                                &format!(
                                    "{}",
                                    character_select_state.create_character_hair_index + 1
                                ),
                            );
                            ui.add_label_in(
                                egui::Rect::from_min_size(
                                    egui::pos2(172.0, 245.0),
                                    egui::vec2(63.0, 19.0),
                                ),
                                CREATE_CHARACTER_STARTPOS_LIST
                                    [character_select_state.create_character_startpos_index],
                            );
                            ui.add_label_in(
                                egui::Rect::from_min_size(
                                    egui::pos2(172.0, 275.0),
                                    egui::vec2(63.0, 19.0),
                                ),
                                CREATE_CHARACTER_BIRTHSTONE_LIST
                                    [character_select_state.create_character_birthstone_index],
                            );
                        },
                    )
                });

            if character_select_state.state != character_select_state.prev_state {
                if let Some(response_editbox) = response_editbox {
                    if !response_editbox.has_focus() {
                        response_editbox.request_focus();
                    }
                }
            }

            if response_prev_sex.map_or(false, |r| r.clicked())
                || response_next_sex.map_or(false, |r| r.clicked())
            {
                if matches!(
                    character_select_state.create_character_gender,
                    CharacterGender::Male
                ) {
                    character_select_state.create_character_gender = CharacterGender::Female;
                } else {
                    character_select_state.create_character_gender = CharacterGender::Male;
                }
            }

            if response_prev_face.map_or(false, |r| r.clicked()) {
                if character_select_state.create_character_face_index == 0 {
                    character_select_state.create_character_face_index =
                        CREATE_CHARACTER_FACE_LIST.len() - 1;
                } else {
                    character_select_state.create_character_face_index -= 1;
                }
            }

            if response_next_face.map_or(false, |r| r.clicked()) {
                character_select_state.create_character_face_index += 1;

                if character_select_state.create_character_face_index
                    == CREATE_CHARACTER_FACE_LIST.len()
                {
                    character_select_state.create_character_face_index = 0;
                }
            }

            if response_prev_hair.map_or(false, |r| r.clicked()) {
                if character_select_state.create_character_hair_index == 0 {
                    character_select_state.create_character_hair_index =
                        CREATE_CHARACTER_HAIR_LIST.len() - 1;
                } else {
                    character_select_state.create_character_hair_index -= 1;
                }
            }

            if response_next_hair.map_or(false, |r| r.clicked()) {
                character_select_state.create_character_hair_index += 1;

                if character_select_state.create_character_hair_index
                    == CREATE_CHARACTER_HAIR_LIST.len()
                {
                    character_select_state.create_character_hair_index = 0;
                }
            }

            if response_prev_birthstone.map_or(false, |r| r.clicked()) {
                if character_select_state.create_character_birthstone_index == 0 {
                    character_select_state.create_character_birthstone_index =
                        CREATE_CHARACTER_BIRTHSTONE_LIST.len() - 1;
                } else {
                    character_select_state.create_character_birthstone_index -= 1;
                }
            }

            if response_next_birthstone.map_or(false, |r| r.clicked()) {
                character_select_state.create_character_birthstone_index += 1;

                if character_select_state.create_character_birthstone_index
                    == CREATE_CHARACTER_BIRTHSTONE_LIST.len()
                {
                    character_select_state.create_character_birthstone_index = 0;
                }
            }

            if response_prev_startpos.map_or(false, |r| r.clicked()) {
                if character_select_state.create_character_startpos_index == 0 {
                    character_select_state.create_character_startpos_index =
                        CREATE_CHARACTER_STARTPOS_LIST.len() - 1;
                } else {
                    character_select_state.create_character_startpos_index -= 1;
                }
            }

            if response_next_startpos.map_or(false, |r| r.clicked()) {
                character_select_state.create_character_startpos_index += 1;

                if character_select_state.create_character_startpos_index
                    == CREATE_CHARACTER_STARTPOS_LIST.len()
                {
                    character_select_state.create_character_startpos_index = 0;
                }
            }

            if response_ok.map_or(false, |r| r.clicked())
                && character_select_state.create_character_name.len() > 3
            {
                if let Some(world_connection) = world_connection.as_ref() {
                    world_connection
                        .client_message_tx
                        .send(ClientMessage::CreateCharacter(CreateCharacter {
                            gender: character_select_state.create_character_gender,
                            birth_stone: character_select_state.create_character_birthstone_index
                                as i32,
                            hair: CREATE_CHARACTER_HAIR_LIST
                                [character_select_state.create_character_hair_index],
                            face: CREATE_CHARACTER_FACE_LIST
                                [character_select_state.create_character_face_index],
                            name: character_select_state.create_character_name.clone(),
                            start_point: character_select_state.create_character_startpos_index
                                as i32,
                            hair_color: 1,
                            weapon_type: 0,
                        }))
                        .ok();
                }
                character_select_state.state = CharacterSelectState::CharacterCreating;
            }

            if response_cancel.map_or(false, |r| r.clicked()) {
                let (camera_entity, _, _, _) = query_camera.single();
                commands
                    .entity(camera_entity)
                    .insert(ActiveMotion::new_once(
                        asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
                    ));
                character_select_state.state = CharacterSelectState::CharacterSelect;
            }

            if !character_select_state
                .create_character_error_message
                .is_empty()
            {
                /*
                // TODO: Show error message box
                  ui.label(
                    egui::RichText::new(format!(
                        "Error: {}",
                        character_select_state.create_character_error_message
                    ))
                    .color(egui::Color32::RED),
                );
                */
            }

            if let Some(create_character_entity) = character_select_state.create_character_entity {
                if let Ok(mut create_character_info) =
                    query_create_character_info.get_mut(create_character_entity)
                {
                    let face = CREATE_CHARACTER_FACE_LIST
                        [character_select_state.create_character_face_index]
                        as u8;
                    let hair = CREATE_CHARACTER_HAIR_LIST
                        [character_select_state.create_character_hair_index]
                        as u8;

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
                                [character_select_state.create_character_face_index]
                                as u8,
                            hair: CREATE_CHARACTER_HAIR_LIST
                                [character_select_state.create_character_hair_index]
                                as u8,
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
        }
        CharacterSelectState::ConnectingGameServer => {
            egui::Window::new("Connecting...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Connecting to game");
                });

            for event in game_connection_events.iter() {
                let &GameConnectionEvent::Connected(zone_id) = event;

                // Start camera animation
                let (camera_entity, _, _, _) = query_camera.single();
                commands
                    .entity(camera_entity)
                    .insert(ActiveMotion::new_once(
                        asset_server.load("3DDATA/TITLE/CAMERA01_INGAME01.ZMO"),
                    ));

                character_select_state.state = CharacterSelectState::Leaving;
                character_select_state.join_zone_id = Some(zone_id);
            }
        }
        CharacterSelectState::Leaving => {
            let (_, _, _, camera_motion) = query_camera.single();
            if camera_motion.is_none() || server_configuration.auto_login {
                // Wait until camera motion complete, then load the zone!
                character_select_state.state = CharacterSelectState::Loading;
                load_zone_events.send(LoadZoneEvent::new(
                    character_select_state.join_zone_id.unwrap(),
                ));
            }
        }
        CharacterSelectState::Loading => {}
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

    for (_, camera, camera_transform, _) in query_camera.iter() {
        if let Some(index) = character_select_state.selected_character_index {
            if let Some(selected_character) = character_list
                .as_ref()
                .and_then(|character_list| character_list.characters.get(index))
            {
                if let Some(screen_pos) = camera.world_to_viewport(
                    camera_transform,
                    Vec3::new(
                        CHARACTER_POSITIONS[index][0],
                        CHARACTER_POSITIONS[index][1] + 4.0,
                        CHARACTER_POSITIONS[index][2],
                    ),
                ) {
                    let screen_max_y = egui_context.ctx_mut().input().screen_rect().max.y;

                    egui::containers::popup::show_tooltip_at(
                        egui_context.ctx_mut(),
                        egui::Id::new("selected_character_plate"),
                        Some(egui::Pos2::new(
                            screen_pos.x - 30.0,
                            screen_max_y - screen_pos.y,
                        )),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&selected_character.info.name)
                                    .font(egui::FontId::proportional(20.0))
                                    .color(if selected_character.delete_time.is_none() {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::RED
                                    }),
                            );
                            ui.label(format!("Level: {}", selected_character.level.level));
                            ui.label(format!("Job: {}", selected_character.info.job));
                            if let Some(delete_time) = selected_character.delete_time.as_ref() {
                                let duration = delete_time.get_time_until_delete();
                                let seconds = duration.as_secs() % 60;
                                let minutes = (duration.as_secs() / 60) % 60;
                                ui.label(format!("Deleted in {:02}m {:02}s", minutes, seconds));
                            }
                        },
                    );
                }
            }
        }
    }

    character_select_state.prev_state = character_select_state.state;
}

#[allow(clippy::too_many_arguments)]
pub fn character_select_input_system(
    mut commands: Commands,
    mut character_select_state: ResMut<CharacterSelect>,
    mut egui_ctx: ResMut<EguiContext>,
    asset_server: Res<AssetServer>,
    mouse_button_input: Res<Input<MouseButton>>,
    rapier_context: Res<RapierContext>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &Projection, &GlobalTransform), With<Camera3d>>,
    query_collider_parent: Query<&ColliderParent>,
    query_select_character: Query<&CharacterSelectCharacter>,
) {
    let cursor_position = windows.primary().cursor_position();
    if cursor_position.is_none() {
        // Mouse not in window
        return;
    }
    let cursor_position = cursor_position.unwrap();

    if egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse is over UI
        return;
    }

    if mouse_button_input.just_pressed(MouseButton::Left) {
        for (camera, camera_projection, camera_transform) in query_camera.iter() {
            if let Some((ray_origin, ray_direction)) = ray_from_screenspace(
                cursor_position,
                &windows,
                camera,
                camera_projection,
                camera_transform,
            ) {
                if let Some((collider_entity, _)) = rapier_context.cast_ray(
                    ray_origin,
                    ray_direction,
                    10000000.0,
                    false,
                    InteractionGroups::all().with_memberships(COLLISION_FILTER_CLICKABLE),
                    None,
                ) {
                    let hit_entity = query_collider_parent
                        .get(collider_entity)
                        .map_or(collider_entity, |collider_parent| collider_parent.entity);

                    if let Ok(select_character) = query_select_character.get(hit_entity) {
                        let now = Instant::now();

                        if character_select_state.selected_character_index
                            == Some(select_character.index)
                        {
                            if let Some(last_selected_time) =
                                character_select_state.last_selected_time
                            {
                                if now - last_selected_time < Duration::from_millis(250) {
                                    character_select_state.try_play_character = true;
                                }
                            }
                        }

                        character_select_state.selected_character_index =
                            Some(select_character.index);
                        character_select_state.last_selected_time = Some(now);

                        commands.entity(hit_entity).insert(ActiveMotion::new_once(
                            asset_server.load("3DDATA/MOTION/AVATAR/EVENT_SELECT_M1.ZMO"),
                        ));
                    }
                }
            }
        }
    }
}
