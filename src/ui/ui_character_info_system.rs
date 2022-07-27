use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

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
    ui::{draw_dialog, Dialog, DialogDataBindings, UiStateWindows},
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
    mut egui_context: ResMut<EguiContext>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    mut ui_state: Local<UiStateCharacterInfo>,
    mut ui_state_windows: ResMut<UiStateWindows>,
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

            draw_dialog(
                ui,
                dialog,
                DialogDataBindings {
                    checked: [],
                    text: [],
                    response: [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_UP_STR, &mut response_raise_str_button),
                        (IID_BTN_UP_DEX, &mut response_raise_dex_button),
                        (IID_BTN_UP_INT, &mut response_raise_int_button),
                        (IID_BTN_UP_CON, &mut response_raise_con_button),
                        (IID_BTN_UP_CHARM, &mut response_raise_cha_button),
                        (IID_BTN_UP_SENSE, &mut response_raise_sen_button),
                    ],
                    gauge: [(
                        IID_GUAGE_STAMINA,
                        &stamina,
                        &format!("{} / {}", player.stamina.stamina, MAX_STAMINA),
                    )],
                    tabs: [(IID_TABBEDPANE, &mut ui_state.current_tab)],
                },
                |ui, bindings| {
                    let draw_text_at = |ui: &mut egui::Ui, x, y, text: &str| {
                        ui.allocate_ui_at_rect(ui.min_rect().translate(egui::vec2(x, y)), |ui| {
                            ui.horizontal_top(|ui| ui.add(egui::Label::new(text))).inner
                        });
                    };

                    match bindings.tab_binding(IID_TABBEDPANE) {
                        Some(&mut IID_TAB_BASICINFO) => {
                            draw_text_at(ui, 59.0, 67.0, &player.character_info.name);
                            draw_text_at(ui, 59.0, 88.0, "TODO: job name");
                            draw_text_at(ui, 59.0, 109.0, "TODO: clan name");
                            draw_text_at(ui, 59.0, 172.0, &format!("{}", player.level.level));
                            draw_text_at(
                                ui,
                                59.0,
                                193.0,
                                &format!("{} / {}", player.experience_points.xp, need_xp),
                            );
                        }
                        Some(&mut IID_TAB_ABILITY) => {
                            draw_text_at(
                                ui,
                                58.0,
                                67.0,
                                &format!("{}", player.ability_values.get_strength()),
                            );
                            draw_text_at(
                                ui,
                                58.0,
                                88.0,
                                &format!("{}", player.ability_values.get_dexterity()),
                            );
                            draw_text_at(
                                ui,
                                58.0,
                                109.0,
                                &format!("{}", player.ability_values.get_intelligence()),
                            );
                            draw_text_at(
                                ui,
                                58.0,
                                130.0,
                                &format!("{}", player.ability_values.get_concentration()),
                            );
                            draw_text_at(
                                ui,
                                58.0,
                                151.0,
                                &format!("{}", player.ability_values.get_charm()),
                            );
                            draw_text_at(
                                ui,
                                58.0,
                                172.0,
                                &format!("{}", player.ability_values.get_sense()),
                            );
                            draw_text_at(
                                ui,
                                69.0,
                                211.0,
                                &format!("{}", player.stat_points.points),
                            );

                            draw_text_at(
                                ui,
                                171.0,
                                67.0,
                                &format!("{}", player.ability_values.get_attack_power()),
                            );
                            draw_text_at(
                                ui,
                                171.0,
                                88.0,
                                &format!("{}", player.ability_values.get_defence()),
                            );
                            draw_text_at(
                                ui,
                                171.0,
                                109.0,
                                &format!("{}", player.ability_values.get_resistance()),
                            );
                            draw_text_at(
                                ui,
                                171.0,
                                130.0,
                                &format!("{}", player.ability_values.get_hit()),
                            );
                            draw_text_at(
                                ui,
                                171.0,
                                151.0,
                                &format!("{}", player.ability_values.get_critical()),
                            );
                            draw_text_at(
                                ui,
                                171.0,
                                172.0,
                                &format!("{}", player.ability_values.get_avoid()),
                            );
                            draw_text_at(
                                ui,
                                171.0,
                                193.0,
                                &format!("{}", player.ability_values.get_attack_speed()),
                            );
                            draw_text_at(ui, 171.0, 214.0, &format!("{}", player.move_speed.speed));
                        }
                        Some(&mut IID_TAB_UNION) => {}
                        _ => {}
                    }
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
                            .send(ClientMessage::IncreaseBasicStat(basic_stat_type))
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
