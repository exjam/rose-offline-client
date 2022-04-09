use bevy::prelude::{Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, ExperiencePoints, Level,
        MoveSpeed, Stamina, StatPoints,
    },
    messages::client::ClientMessage,
};

use crate::{
    components::PlayerCharacter,
    resources::{GameConnection, GameData},
    ui::UiStateWindows,
};

#[derive(PartialEq)]
enum CharacterInfoPage {
    Info,
    Stats,
}

pub struct UiStateCharacterInfo {
    current_page: CharacterInfoPage,
}

impl Default for UiStateCharacterInfo {
    fn default() -> Self {
        Self {
            current_page: CharacterInfoPage::Info,
        }
    }
}

pub fn ui_character_info_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_character_info: Local<UiStateCharacterInfo>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<
        (
            &CharacterInfo,
            &AbilityValues,
            &BasicStats,
            &ExperiencePoints,
            &Level,
            &MoveSpeed,
            &Stamina,
            &StatPoints,
        ),
        With<PlayerCharacter>,
    >,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    let (
        player_character_info,
        player_ability_values,
        player_basic_stats,
        player_experience_points,
        player_level,
        player_move_speed,
        player_stamina,
        player_stat_points,
    ) = query_player.single();

    egui::Window::new("Character Info")
        .id(ui_state_windows.character_info_window_id)
        .open(&mut ui_state_windows.character_info_open)
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_character_info.current_page,
                    CharacterInfoPage::Info,
                    "Info",
                );
                ui.selectable_value(
                    &mut ui_state_character_info.current_page,
                    CharacterInfoPage::Stats,
                    "Stats",
                );
            });

            match ui_state_character_info.current_page {
                CharacterInfoPage::Info => {
                    egui::Grid::new("info_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Name");
                        ui.label(&player_character_info.name);
                        ui.end_row();

                        ui.label("Job");
                        ui.label(format!("{}", player_character_info.job));
                        ui.end_row();

                        ui.label("Clan");
                        ui.label("");
                        ui.end_row();

                        ui.label("Level");
                        ui.label(format!("{}", player_level.level));
                        ui.end_row();

                        ui.label("Experience");
                        ui.scope(|ui| {
                            let need_xp = game_data
                                .ability_value_calculator
                                .calculate_levelup_require_xp(player_level.level);
                            ui.style_mut().visuals.selection.bg_fill =
                                egui::Color32::from_rgb(145, 133, 0);
                            ui.add(
                                egui::ProgressBar::new(
                                    player_experience_points.xp as f32 / need_xp as f32,
                                )
                                .text(format!("{} / {}", player_experience_points.xp, need_xp)),
                            );
                        });
                        ui.end_row();

                        ui.label("Stamina");
                        ui.scope(|ui| {
                            ui.style_mut().visuals.selection.bg_fill =
                                egui::Color32::from_rgb(145, 133, 0);
                            ui.add(
                                egui::ProgressBar::new(player_stamina.stamina as f32 / 5000.0)
                                    .text(format!("{} / {}", player_stamina.stamina, 5000.0)),
                            );
                        });
                        ui.end_row();
                    });
                }
                CharacterInfoPage::Stats => {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                        egui::Grid::new("basic_stats_grid")
                            .num_columns(3)
                            .show(ui, |ui| {
                                let show_stat = |ui: &mut egui::Ui, name: &str, value: i32, basic_stat_type: BasicStatType| {
                                    ui.label(name);
                                    ui.label(format!("{}", value));
                                    if let Some(cost) = game_data
                                        .ability_value_calculator
                                        .calculate_basic_stat_increase_cost(
                                            player_basic_stats,
                                            basic_stat_type,
                                        )
                                    {
                                        if ui.add_enabled(cost <= player_stat_points.points, egui::Button::new("+"))
                                            .on_hover_text(format!("Required Points: {}", cost))
                                            .clicked()
                                        {
                                            if let Some(game_connection) = game_connection.as_ref() {
                                                game_connection.client_message_tx.send(ClientMessage::IncreaseBasicStat(basic_stat_type)).ok();
                                            }
                                        }
                                    }
                                    ui.end_row();
                                };

                                show_stat(ui, "Strength", player_basic_stats.strength, BasicStatType::Strength);
                                show_stat(ui, "Dexterity", player_basic_stats.dexterity, BasicStatType::Dexterity);
                                show_stat(ui, "Intelligence", player_basic_stats.intelligence, BasicStatType::Intelligence);
                                show_stat(ui, "Concentration", player_basic_stats.concentration, BasicStatType::Concentration);
                                show_stat(ui, "Charm", player_basic_stats.charm, BasicStatType::Charm);
                                show_stat(ui, "Sense", player_basic_stats.sense, BasicStatType::Sense);
                            });

                            ui.label(format!("Available Points: {}", player_stat_points.points));
                        });

                        ui.separator();
                        egui::Grid::new("ability_values_grid")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Attack");
                                ui.label(format!("{}", player_ability_values.get_attack_power()));
                                ui.end_row();

                                ui.label("Defence");
                                ui.label(format!("{}", player_ability_values.get_defence()));
                                ui.end_row();

                                ui.label("Magic Defence");
                                ui.label(format!("{}", player_ability_values.get_resistance()));
                                ui.end_row();

                                ui.label("Hit");
                                ui.label(format!("{}", player_ability_values.get_hit()));
                                ui.end_row();

                                ui.label("Critical");
                                ui.label(format!("{}", player_ability_values.get_critical()));
                                ui.end_row();

                                ui.label("Avoid");
                                ui.label(format!("{}", player_ability_values.get_avoid()));
                                ui.end_row();

                                ui.label("Attack Speed");
                                ui.label(format!("{}", player_ability_values.get_attack_speed()));
                                ui.end_row();

                                ui.label("Move Speed");
                                ui.label(format!("{}", player_move_speed.speed));
                                ui.end_row();
                            });
                    });
                }
            }
        });
}
