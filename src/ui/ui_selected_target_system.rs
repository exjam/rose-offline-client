use bevy::prelude::{Commands, Entity, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use rose_game_common::components::{AbilityValues, CharacterInfo, HealthPoints, Npc};

use crate::{
    components::{PlayerCharacter, SelectedTarget},
    resources::GameData,
};

pub fn ui_selected_target_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    game_data: Res<GameData>,
    query_player: Query<(Entity, Option<&SelectedTarget>), With<PlayerCharacter>>,
    query_target: Query<(
        &AbilityValues,
        &HealthPoints,
        Option<&Npc>,
        Option<&CharacterInfo>,
    )>,
) {
    let (player_entity, player_target) = query_player.single();

    if let Some(player_target) = player_target {
        if let Ok((ability_values, health_points, npc, character_info)) =
            query_target.get(player_target.entity)
        {
            egui::Window::new("Selected Target")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
                .collapsible(false)
                .title_bar(false)
                .show(egui_context.ctx_mut(), |ui| {
                    if let Some(npc_data) = npc.and_then(|npc| game_data.npcs.get_npc(npc.id)) {
                        ui.label(&npc_data.name);
                    } else if let Some(character_info) = character_info {
                        ui.label(&character_info.name);
                    } else {
                        ui.label("???");
                    }

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
