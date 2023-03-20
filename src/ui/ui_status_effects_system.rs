use std::time::Duration;

use bevy::{
    ecs::query::WorldQuery,
    prelude::{Entity, Query, Res, With},
    time::Time,
};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::components::StatusEffects;

use crate::{
    components::PlayerCharacter,
    resources::{GameData, UiResources, UiSpriteSheetType},
};

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    status_effects: &'w StatusEffects,
}

pub fn ui_status_effects_system(
    mut egui_context: EguiContexts,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    time: Res<Time>,
) {
    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    egui::Window::new("Player Status Effects}")
        .anchor(egui::Align2::LEFT_TOP, [250.0, 40.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal_top(|ui| {
                for (status_effect_type, active_status_effect) in
                    player.status_effects.active.iter()
                {
                    if let Some(active_status_effect) = active_status_effect {
                        if let Some(status_effect_data) = game_data
                            .status_effects
                            .get_status_effect(active_status_effect.id)
                        {
                            let remaining_time = if let Some(expire_time) =
                                player.status_effects.expire_times[status_effect_type]
                            {
                                let now = time.last_update().unwrap();
                                if now >= expire_time {
                                    Some(Duration::ZERO)
                                } else {
                                    Some(expire_time - now)
                                }
                            } else {
                                None
                            };

                            if let Some(sprite) = ui_resources.get_sprite_by_index(
                                UiSpriteSheetType::StateIcon,
                                status_effect_data.icon_id as usize,
                            ) {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(sprite.width, sprite.height),
                                    egui::Sense::hover(),
                                );
                                sprite.draw(ui, rect.min);

                                if response.hovered() {
                                    if let Some(remaining_time) = remaining_time {
                                        response.on_hover_text(format!(
                                            "{}\n\nTime Remaining: {} seconds",
                                            status_effect_data.name,
                                            remaining_time.as_secs()
                                        ));
                                    } else {
                                        response.on_hover_text(status_effect_data.name);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
}
