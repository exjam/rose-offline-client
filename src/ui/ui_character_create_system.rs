use bevy::prelude::{
    AssetServer, Assets, Camera3d, Commands, ComputedVisibility, DespawnRecursiveExt, Entity,
    GlobalTransform, Local, Quat, Query, Res, ResMut, Transform, Vec3, Visibility, With,
};
use bevy_egui::{egui, EguiContext};
use rose_data::ZoneId;
use rose_game_common::{
    components::{CharacterGender, CharacterInfo, Equipment},
    messages::client::{ClientMessage, CreateCharacter},
};

use crate::{
    components::ActiveMotion,
    resources::{CharacterSelectState, UiResources, WorldConnection},
    ui::widgets::{DataBindings, Dialog},
};

use super::widgets::DrawText;

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

pub struct UiCharacterCreateState {
    initial_focus_set: bool,
    entity: Option<Entity>,
    name: String,
    gender: CharacterGender,
    hair_index: usize,
    face_index: usize,
    startpos_index: usize,
    birthstone_index: usize,
    error_message: String,
}

impl Default for UiCharacterCreateState {
    fn default() -> Self {
        Self {
            initial_focus_set: false,
            entity: None,
            name: String::new(),
            gender: CharacterGender::Male,
            hair_index: 0,
            face_index: 0,
            startpos_index: 0,
            birthstone_index: 0,
            error_message: String::new(),
        }
    }
}

pub fn ui_character_create_system(
    mut commands: Commands,
    mut ui_state: Local<UiCharacterCreateState>,
    mut character_select_state: ResMut<CharacterSelectState>,
    mut egui_context: ResMut<EguiContext>,
    query_camera: Query<Entity, With<Camera3d>>,
    mut query_create_character_info: Query<&mut CharacterInfo>,
    asset_server: Res<AssetServer>,
    dialog_assets: Res<Assets<Dialog>>,
    ui_resources: Res<UiResources>,
    world_connection: Option<Res<WorldConnection>>,
) {
    let ui_state = &mut *ui_state;
    if !matches!(
        *character_select_state,
        CharacterSelectState::CharacterCreate
    ) {
        if let Some(entity) = ui_state.entity.take() {
            commands.entity(entity).despawn_recursive();
        }

        ui_state.initial_focus_set = false;
        return;
    }

    let world_connection = if let Some(world_connection) = world_connection {
        world_connection
    } else {
        return;
    };

    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_create_avatar) {
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

    let screen_size = egui_context.ctx_mut().input().screen_rect().size();

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
                    text: &mut [(IID_EDITBOX, &mut ui_state.name)],
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
                        egui::Rect::from_min_size(egui::pos2(172.0, 155.0), egui::vec2(63.0, 19.0)),
                        match ui_state.gender {
                            CharacterGender::Male => "Male",
                            CharacterGender::Female => "Female",
                        },
                    );
                    ui.add_label_in(
                        egui::Rect::from_min_size(egui::pos2(172.0, 185.0), egui::vec2(63.0, 19.0)),
                        &format!("{}", ui_state.face_index + 1),
                    );
                    ui.add_label_in(
                        egui::Rect::from_min_size(egui::pos2(172.0, 215.0), egui::vec2(63.0, 19.0)),
                        &format!("{}", ui_state.hair_index + 1),
                    );
                    ui.add_label_in(
                        egui::Rect::from_min_size(egui::pos2(172.0, 245.0), egui::vec2(63.0, 19.0)),
                        CREATE_CHARACTER_STARTPOS_LIST[ui_state.startpos_index],
                    );
                    ui.add_label_in(
                        egui::Rect::from_min_size(egui::pos2(172.0, 275.0), egui::vec2(63.0, 19.0)),
                        CREATE_CHARACTER_BIRTHSTONE_LIST[ui_state.birthstone_index],
                    );
                },
            )
        });

    if !ui_state.initial_focus_set {
        if let Some(response_editbox) = response_editbox {
            if !response_editbox.has_focus() {
                response_editbox.request_focus();
            }
            ui_state.initial_focus_set = true;
        }
    }

    if response_prev_sex.map_or(false, |r| r.clicked())
        || response_next_sex.map_or(false, |r| r.clicked())
    {
        if matches!(ui_state.gender, CharacterGender::Male) {
            ui_state.gender = CharacterGender::Female;
        } else {
            ui_state.gender = CharacterGender::Male;
        }
    }

    if response_prev_face.map_or(false, |r| r.clicked()) {
        if ui_state.face_index == 0 {
            ui_state.face_index = CREATE_CHARACTER_FACE_LIST.len() - 1;
        } else {
            ui_state.face_index -= 1;
        }
    }

    if response_next_face.map_or(false, |r| r.clicked()) {
        ui_state.face_index += 1;

        if ui_state.face_index == CREATE_CHARACTER_FACE_LIST.len() {
            ui_state.face_index = 0;
        }
    }

    if response_prev_hair.map_or(false, |r| r.clicked()) {
        if ui_state.hair_index == 0 {
            ui_state.hair_index = CREATE_CHARACTER_HAIR_LIST.len() - 1;
        } else {
            ui_state.hair_index -= 1;
        }
    }

    if response_next_hair.map_or(false, |r| r.clicked()) {
        ui_state.hair_index += 1;

        if ui_state.hair_index == CREATE_CHARACTER_HAIR_LIST.len() {
            ui_state.hair_index = 0;
        }
    }

    if response_prev_birthstone.map_or(false, |r| r.clicked()) {
        if ui_state.birthstone_index == 0 {
            ui_state.birthstone_index = CREATE_CHARACTER_BIRTHSTONE_LIST.len() - 1;
        } else {
            ui_state.birthstone_index -= 1;
        }
    }

    if response_next_birthstone.map_or(false, |r| r.clicked()) {
        ui_state.birthstone_index += 1;

        if ui_state.birthstone_index == CREATE_CHARACTER_BIRTHSTONE_LIST.len() {
            ui_state.birthstone_index = 0;
        }
    }

    if response_prev_startpos.map_or(false, |r| r.clicked()) {
        if ui_state.startpos_index == 0 {
            ui_state.startpos_index = CREATE_CHARACTER_STARTPOS_LIST.len() - 1;
        } else {
            ui_state.startpos_index -= 1;
        }
    }

    if response_next_startpos.map_or(false, |r| r.clicked()) {
        ui_state.startpos_index += 1;

        if ui_state.startpos_index == CREATE_CHARACTER_STARTPOS_LIST.len() {
            ui_state.startpos_index = 0;
        }
    }

    if response_ok.map_or(false, |r| r.clicked()) && ui_state.name.len() > 3 {
        world_connection
            .client_message_tx
            .send(ClientMessage::CreateCharacter(CreateCharacter {
                gender: ui_state.gender,
                birth_stone: ui_state.birthstone_index as i32,
                hair: CREATE_CHARACTER_HAIR_LIST[ui_state.hair_index],
                face: CREATE_CHARACTER_FACE_LIST[ui_state.face_index],
                name: ui_state.name.clone(),
                start_point: ui_state.startpos_index as i32,
                hair_color: 1,
                weapon_type: 0,
            }))
            .ok();

        *character_select_state = CharacterSelectState::CharacterCreating;
    }

    if response_cancel.map_or(false, |r| r.clicked()) {
        commands
            .entity(query_camera.single())
            .insert(ActiveMotion::new_once(
                asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
            ));
        *character_select_state = CharacterSelectState::CharacterSelect(None);
    }

    if !ui_state.error_message.is_empty() {
        /*
        // TODO: Show error message box
          ui.label(
            egui::RichText::new(format!(
                "Error: {}",
                ui_state.error_message
            ))
            .color(egui::Color32::RED),
        );
        */
    }

    if let Some(create_character_entity) = ui_state.entity {
        if let Ok(mut create_character_info) =
            query_create_character_info.get_mut(create_character_entity)
        {
            let face = CREATE_CHARACTER_FACE_LIST[ui_state.face_index] as u8;
            let hair = CREATE_CHARACTER_HAIR_LIST[ui_state.hair_index] as u8;

            if create_character_info.face != face {
                create_character_info.face = face;
            }

            if create_character_info.hair != hair {
                create_character_info.hair = hair;
            }

            if create_character_info.gender != ui_state.gender {
                create_character_info.gender = ui_state.gender;
            }
        }
    } else {
        let create_character_entity = commands
            .spawn((
                CharacterInfo {
                    name: "CreateCharacter".to_string(),
                    gender: ui_state.gender,
                    race: 0,
                    birth_stone: 0,
                    job: 0,
                    face: CREATE_CHARACTER_FACE_LIST[ui_state.face_index] as u8,
                    hair: CREATE_CHARACTER_HAIR_LIST[ui_state.hair_index] as u8,
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
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .id();
        ui_state.entity = Some(create_character_entity);
    }
}
