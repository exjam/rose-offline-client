use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, EventWriter, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, ExperiencePoints, Level,
        MoveSpeed, Stamina, StatPoints, MAX_STAMINA,
    },
    messages::client::ClientMessage,
};

use crate::{
    components::PlayerCharacter,
    resources::{GameConnection, GameData, UiResources},
    ui::{
        widgets::{DataBindings, Dialog, DrawText},
        UiSoundEvent, UiStateWindows,
    },
};

const IID_BTN_CLOSE: i32 = 10;
// const IID_BTN_DIALOG2ICON: i32 = 11;
const IID_TABBEDPANE: i32 = 20;
const IID_TAB_BASICINFO: i32 = 21;
// const IID_TAB_BASICINFO_BG: i32 = 22;
// const IID_TAB_BASICINFO_BTN: i32 = 23;
const IID_GUAGE_STAMINA: i32 = 24;
const IID_TAB_ABILITY: i32 = 31;
// const IID_TAB_ABILITY_BG: i32 = 32;
// const IID_TAB_ABILITY_BTN: i32 = 33;
const IID_BTN_UP_STR: i32 = 34;
const IID_BTN_UP_DEX: i32 = 35;
const IID_BTN_UP_INT: i32 = 36;
const IID_BTN_UP_CON: i32 = 37;
const IID_BTN_UP_CHARM: i32 = 38;
const IID_BTN_UP_SENSE: i32 = 39;
const IID_TAB_UNION: i32 = 41;
// const IID_TAB_UNION_BG: i32 = 42;
// const IID_TAB_UNION_BTN: i32 = 43;

pub struct UiStateCharacterInfo {
    current_tab: i32,
}

impl Default for UiStateCharacterInfo {
    fn default() -> Self {
        Self {
            current_tab: IID_TAB_BASICINFO,
        }
    }
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    ability_values: &'w AbilityValues,
    basic_stats: &'w BasicStats,
    character_info: &'w CharacterInfo,
    experience_points: &'w ExperiencePoints,
    level: &'w Level,
    move_speed: &'w MoveSpeed,
    stamina: &'w Stamina,
    stat_points: &'w StatPoints,
}

pub fn ui_character_info_system(
    mut egui_context: EguiContexts,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    mut ui_state: Local<UiStateCharacterInfo>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_character_info) {
        dialog
    } else {
        return;
    };

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    let ui_state = &mut *ui_state;
    let mut response_close_button = None;
    let mut response_raise_str_button = None;
    let mut response_raise_dex_button = None;
    let mut response_raise_int_button = None;
    let mut response_raise_con_button = None;
    let mut response_raise_cha_button = None;
    let mut response_raise_sen_button = None;

    egui::Window::new("Character Info")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.character_info_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            let need_xp = game_data
                .ability_value_calculator
                .calculate_levelup_require_xp(player.level.level);
            let stamina = player.stamina.stamina as f32 / MAX_STAMINA as f32;

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_UP_STR, &mut response_raise_str_button),
                        (IID_BTN_UP_DEX, &mut response_raise_dex_button),
                        (IID_BTN_UP_INT, &mut response_raise_int_button),
                        (IID_BTN_UP_CON, &mut response_raise_con_button),
                        (IID_BTN_UP_CHARM, &mut response_raise_cha_button),
                        (IID_BTN_UP_SENSE, &mut response_raise_sen_button),
                    ],
                    gauge: &mut [(
                        IID_GUAGE_STAMINA,
                        &stamina,
                        &format!("{} / {}", player.stamina.stamina, MAX_STAMINA),
                    )],
                    tabs: &mut [(IID_TABBEDPANE, &mut ui_state.current_tab)],
                    ..Default::default()
                },
                |ui, bindings| match bindings.get_tab(IID_TABBEDPANE) {
                    Some(&mut IID_TAB_BASICINFO) => {
                        ui.add_label_at(egui::pos2(59.0, 67.0), &player.character_info.name);
                        ui.add_label_at(
                            egui::pos2(59.0, 88.0),
                            game_data
                                .string_database
                                .get_job_name(player.character_info.job),
                        );
                        // ui.add_label_at(egui::pos2(59.0, 109.0), ""); // TODO: Clan name
                        ui.add_label_at(
                            egui::pos2(59.0, 172.0),
                            &format!("{}", player.level.level),
                        );
                        ui.add_label_at(
                            egui::pos2(59.0, 193.0),
                            &format!("{} / {}", player.experience_points.xp, need_xp),
                        );
                    }
                    Some(&mut IID_TAB_ABILITY) => {
                        ui.add_label_at(
                            egui::pos2(58.0, 67.0),
                            &format!("{}", player.ability_values.get_strength()),
                        );
                        ui.add_label_at(
                            egui::pos2(58.0, 88.0),
                            &format!("{}", player.ability_values.get_dexterity()),
                        );
                        ui.add_label_at(
                            egui::pos2(58.0, 109.0),
                            &format!("{}", player.ability_values.get_intelligence()),
                        );
                        ui.add_label_at(
                            egui::pos2(58.0, 130.0),
                            &format!("{}", player.ability_values.get_concentration()),
                        );
                        ui.add_label_at(
                            egui::pos2(58.0, 151.0),
                            &format!("{}", player.ability_values.get_charm()),
                        );
                        ui.add_label_at(
                            egui::pos2(58.0, 172.0),
                            &format!("{}", player.ability_values.get_sense()),
                        );
                        ui.add_label_at(
                            egui::pos2(69.0, 211.0),
                            &format!("{}", player.stat_points.points),
                        );

                        ui.add_label_at(
                            egui::pos2(171.0, 67.0),
                            &format!("{}", player.ability_values.get_attack_power()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 88.0),
                            &format!("{}", player.ability_values.get_defence()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 109.0),
                            &format!("{}", player.ability_values.get_resistance()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 130.0),
                            &format!("{}", player.ability_values.get_hit()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 151.0),
                            &format!("{}", player.ability_values.get_critical()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 172.0),
                            &format!("{}", player.ability_values.get_avoid()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 193.0),
                            &format!("{}", player.ability_values.get_attack_speed()),
                        );
                        ui.add_label_at(
                            egui::pos2(171.0, 214.0),
                            &format!("{}", player.move_speed.speed),
                        );
                    }
                    Some(&mut IID_TAB_UNION) => {}
                    _ => {}
                },
            );
        });

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.character_info_open = false;
    }

    let stat_button_response = |basic_stat_type: BasicStatType,
                                response: Option<egui::Response>| {
        if let Some(response) = response {
            if let Some(cost) = game_data
                .ability_value_calculator
                .calculate_basic_stat_increase_cost(player.basic_stats, basic_stat_type)
            {
                if response
                    .on_hover_text(format!("Required Points: {}", cost))
                    .clicked()
                    && cost <= player.stat_points.points
                {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::IncreaseBasicStat { basic_stat_type })
                            .ok();
                    }
                }
            }
        }
    };

    stat_button_response(BasicStatType::Strength, response_raise_str_button);
    stat_button_response(BasicStatType::Dexterity, response_raise_dex_button);
    stat_button_response(BasicStatType::Intelligence, response_raise_int_button);
    stat_button_response(BasicStatType::Concentration, response_raise_con_button);
    stat_button_response(BasicStatType::Charm, response_raise_cha_button);
    stat_button_response(BasicStatType::Sense, response_raise_sen_button);
}
