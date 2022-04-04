use bevy::prelude::{Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use rose_game_common::components::{
    AbilityValues, CharacterInfo, ExperiencePoints, HealthPoints, Level, ManaPoints,
};

use crate::{components::PlayerCharacter, resources::GameData};

pub fn ui_player_info_system(
    mut egui_context: ResMut<EguiContext>,
    query_player: Query<
        (
            &AbilityValues,
            &CharacterInfo,
            &Level,
            &HealthPoints,
            &ManaPoints,
            &ExperiencePoints,
        ),
        With<PlayerCharacter>,
    >,
    game_data: Res<GameData>,
) {
    let (
        player_ability_values,
        player_info,
        player_level,
        player_health_points,
        player_mana_points,
        player_experience_points,
    ) = query_player.single();

    egui::Window::new("Player Info")
        .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
        .collapsible(false)
        .title_bar(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.label(&player_info.name);

            egui::Grid::new("player_info_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("HP");
                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_RED;
                        ui.add(
                            egui::ProgressBar::new(
                                player_health_points.hp as f32
                                    / player_ability_values.get_max_health() as f32,
                            )
                            .text(format!(
                                "{} / {}",
                                player_health_points.hp,
                                player_ability_values.get_max_health()
                            )),
                        )
                    });
                    ui.end_row();

                    ui.label("MP");
                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_BLUE;
                        ui.add(
                            egui::ProgressBar::new(
                                player_mana_points.mp as f32
                                    / player_ability_values.get_max_mana() as f32,
                            )
                            .text(format!(
                                "{} / {}",
                                player_mana_points.mp,
                                player_ability_values.get_max_mana()
                            )),
                        );
                    });
                    ui.end_row();

                    ui.label("XP");
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
                            .show_percentage(),
                        )
                        .on_hover_text(format!("{} / {}", player_experience_points.xp, need_xp));
                    });
                    ui.end_row();
                });

            ui.label(format!("Level {}", player_level.level));
        });
}
