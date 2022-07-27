use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, Commands, Entity, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};
use rose_game_common::components::{
    AbilityValues, CharacterInfo, ExperiencePoints, HealthPoints, Level, ManaPoints,
};

use crate::{
    components::{PlayerCharacter, SelectedTarget},
    resources::{GameData, UiResources},
    ui::{draw_dialog, Dialog, DialogDataBindings},
};

const IID_GAUGE_HP: i32 = 6;
const IID_GAUGE_MP: i32 = 7;
const IID_GAUGE_EXP: i32 = 8;

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    ability_values: &'w AbilityValues,
    character_info: &'w CharacterInfo,
    level: &'w Level,
    health_points: &'w HealthPoints,
    mana_points: &'w ManaPoints,
    experience_points: &'w ExperiencePoints,
}

pub fn ui_player_info_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_player_info) {
        dialog
    } else {
        return;
    };

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    let response = egui::Window::new("Player Info")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            let hp = player.health_points.hp as f32 / player.ability_values.get_max_health() as f32;
            let mp = player.mana_points.mp as f32 / player.ability_values.get_max_mana() as f32;
            let need_xp = game_data
                .ability_value_calculator
                .calculate_levelup_require_xp(player.level.level);
            let xp = player.experience_points.xp as f32 / need_xp as f32;

            draw_dialog(
                ui,
                dialog,
                DialogDataBindings {
                    checked: [],
                    text: [],
                    response: [],
                    gauge: [
                        (
                            IID_GAUGE_HP,
                            &hp,
                            &format!(
                                "{}/{}",
                                player.health_points.hp,
                                player.ability_values.get_max_health()
                            ),
                        ),
                        (
                            IID_GAUGE_MP,
                            &mp,
                            &format!(
                                "{}/{}",
                                player.mana_points.mp,
                                player.ability_values.get_max_mana()
                            ),
                        ),
                        (IID_GAUGE_EXP, &xp, &format!("{:.2}%", xp * 100.0)),
                    ],
                    tabs: [],
                },
                |ui, _| {
                    ui.put(
                        egui::Rect::from_min_max(
                            ui.min_rect().min + egui::vec2(15.0, 8.0),
                            ui.min_rect().min + egui::vec2(150.0, 25.0),
                        ),
                        egui::Label::new(
                            egui::RichText::new(&player.character_info.name)
                                .color(egui::Color32::from_rgb(0, 255, 42)),
                        ),
                    );
                    ui.put(
                        egui::Rect::from_min_max(
                            ui.min_rect().min + egui::vec2(180.0, 8.0),
                            ui.min_rect().min + egui::vec2(230.0, 25.0),
                        ),
                        egui::Label::new(
                            egui::RichText::new(&format!("{}", player.level.level))
                                .color(egui::Color32::YELLOW),
                        ),
                    );
                },
            )
        });

    if let Some(response) = response {
        if response.response.clicked() {
            commands
                .entity(player.entity)
                .insert(SelectedTarget::new(player.entity));
        }
    }
}
