use bevy::prelude::{Commands, Entity, Query, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_game_common::components::{AbilityValues, HealthPoints};

use crate::components::{ClientEntityName, PlayerCharacter, SelectedTarget};

pub fn ui_selected_target_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    query_player: Query<(Entity, Option<&SelectedTarget>), With<PlayerCharacter>>,
    query_target: Query<(&AbilityValues, &ClientEntityName, &HealthPoints)>,
) {
    let (player_entity, player_target) = query_player.single();

    if let Some(player_target) = player_target {
        if let Ok((ability_values, client_entity_name, health_points)) =
            query_target.get(player_target.entity)
        {
            egui::Window::new("Selected Target")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
                .collapsible(false)
                .title_bar(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label(client_entity_name.as_str());
                    ui.label(format!("Level: {}", ability_values.level));

                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_RED;
                        ui.add(
                            egui::ProgressBar::new(
                                health_points.hp as f32 / ability_values.get_max_health() as f32,
                            )
                            .show_percentage(),
                        )
                        .on_hover_text(format!(
                            "{} / {}",
                            health_points.hp,
                            ability_values.get_max_health()
                        ));
                    });
                });
        } else {
            // Selected target no longer valid, remove it
            commands.entity(player_entity).remove::<SelectedTarget>();
        }
    }
}
