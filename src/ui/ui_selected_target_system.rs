use bevy::prelude::{Commands, Entity, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_game_common::components::{AbilityValues, HealthPoints, Npc};

use crate::{
    components::{ClientEntityName, Command, PlayerCharacter, SelectedTarget},
    resources::{UiResources, UiSprite},
};

#[derive(Default)]
pub struct UiSelectedTargetState {
    pub sprite_top: Option<UiSprite>,
    pub sprite_middle: Option<UiSprite>,
    pub sprite_bottom: Option<UiSprite>,
    pub hp_gauge_background: Option<UiSprite>,
    pub hp_gauge_foreground: Option<UiSprite>,
}

pub fn ui_selected_target_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiSelectedTargetState>,
    query_player: Query<(Entity, Option<&SelectedTarget>), With<PlayerCharacter>>,
    query_target: Query<(
        &AbilityValues,
        &Command,
        &ClientEntityName,
        &HealthPoints,
        Option<&Npc>,
    )>,
    ui_resources: Res<UiResources>,
) {
    let (player_entity, player_target) = query_player.single();

    if ui_state.sprite_top.is_none() {
        ui_state.sprite_top = ui_resources.get_sprite(0, "UI18_PARTYOPTION_TOP");
        ui_state.sprite_middle = ui_resources.get_sprite(0, "UI18_PARTYOPTION_MIDDLE");
        ui_state.sprite_bottom = ui_resources.get_sprite(0, "UI18_PARTYOPTION_BOTTOM");
        ui_state.hp_gauge_background = ui_resources.get_sprite(0, "UI00_GUAGE_BACKGROUND");
        ui_state.hp_gauge_foreground = ui_resources.get_sprite(0, "UI00_GUAGE_RED");
    }

    if let Some(player_target) = player_target {
        if let Ok((ability_values, command, client_entity_name, health_points, npc)) =
            query_target.get(player_target.entity)
        {
            if command.is_die() && npc.is_some() {
                // Cannot target dead NPC
                commands.entity(player_entity).remove::<SelectedTarget>();
            } else {
                egui::Window::new("Selected Target")
                    .anchor(egui::Align2::CENTER_TOP, [0.0, 0.0])
                    .frame(egui::Frame::none())
                    .title_bar(false)
                    .resizable(false)
                    .show(egui_context.ctx_mut(), |ui| {
                        let style = ui.style_mut();
                        style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::BLACK;
                        style.spacing.item_spacing = egui::Vec2::ZERO;
                        style.spacing.window_margin = egui::style::Margin::same(0.0);

                        if let (
                            Some(sprite_top),
                            Some(sprite_middle),
                            Some(sprite_bottom),
                            Some(hp_gauge_background),
                            Some(hp_gauge_foreground),
                        ) = (
                            ui_state.sprite_top.as_ref(),
                            ui_state.sprite_middle.as_ref(),
                            ui_state.sprite_bottom.as_ref(),
                            ui_state.hp_gauge_background.as_ref(),
                            ui_state.hp_gauge_foreground.as_ref(),
                        ) {
                            let size = egui::vec2(
                                sprite_middle.width,
                                sprite_top.height + sprite_middle.height + sprite_bottom.height,
                            );
                            let rect = egui::Rect::from_min_size(ui.min_rect().min, size);
                            ui.allocate_rect(rect, egui::Sense::hover());

                            if ui.is_rect_visible(rect) {
                                sprite_top.draw(ui, rect.min);
                                sprite_middle
                                    .draw(ui, rect.min + egui::vec2(0.0, sprite_top.height - 1.0));
                                sprite_bottom.draw(
                                    ui,
                                    rect.min
                                        + egui::vec2(
                                            0.0,
                                            sprite_top.height + sprite_middle.height - 2.0,
                                        ),
                                );

                                let hp_percent = health_points.hp as f32
                                    / ability_values.get_max_health() as f32;

                                // HP gauge background
                                let gauge_rect = egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + rect.width() / 2.0
                                            - hp_gauge_background.width / 2.0,
                                        rect.max.y - 20.0,
                                    ),
                                    egui::vec2(
                                        hp_gauge_background.width,
                                        hp_gauge_background.height,
                                    ),
                                );
                                hp_gauge_background.draw_stretched(ui, gauge_rect);

                                // HP gauge foreground
                                let mut fg_gauge_rect = gauge_rect;
                                fg_gauge_rect.set_width(hp_gauge_foreground.width * hp_percent);
                                let mut mesh = egui::epaint::Mesh::with_texture(
                                    hp_gauge_foreground.texture_id,
                                );
                                let mut uv = hp_gauge_foreground.uv;
                                uv.max.x *= hp_percent;
                                mesh.add_rect_with_uv(fg_gauge_rect, uv, egui::Color32::WHITE);
                                ui.painter().add(egui::epaint::Shape::mesh(mesh));

                                let hp_text = format!(
                                    "{} / {}",
                                    health_points.hp,
                                    ability_values.get_max_health()
                                );
                                ui.put(
                                    gauge_rect.translate(egui::vec2(1.0, 1.0)),
                                    egui::Label::new(&hp_text),
                                );
                                ui.put(
                                    gauge_rect,
                                    egui::Label::new(
                                        egui::RichText::new(&hp_text).color(egui::Color32::WHITE),
                                    ),
                                );

                                let mut text_rect = rect;
                                text_rect.set_height(20.0);
                                text_rect.min.y += 11.0;
                                text_rect.max.y += 11.0;
                                ui.put(text_rect, egui::Label::new(client_entity_name.as_str()));

                                text_rect.min.y += 14.0;
                                text_rect.max.y += 14.0;
                                ui.put(
                                    text_rect,
                                    egui::Label::new(format!("Level: {}", ability_values.level)),
                                );
                            }
                        }
                    });
            }
        } else {
            // Selected target no longer valid, remove it
            commands.entity(player_entity).remove::<SelectedTarget>();
        }
    }
}
