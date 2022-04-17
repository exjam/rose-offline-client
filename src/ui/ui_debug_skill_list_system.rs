use bevy::prelude::{Local, Res, ResMut, State};
use bevy_egui::{egui, EguiContext};

use rose_game_common::messages::client::ClientMessage;

use crate::{
    resources::{AppState, GameConnection, GameData, Icons},
    ui::{ui_add_skill_tooltip, UiStateDebugWindows},
};

#[derive(Default)]
pub struct UiStateDebugSkillList {
    name_filter: String,
}

pub fn ui_debug_skill_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_skill_list: Local<UiStateDebugSkillList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Skill List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.skill_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("skill_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Skill Name Filter:");
                    ui.text_edit_singleline(&mut ui_state_debug_skill_list.name_filter);
                    ui.end_row();
                });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .always_show_scroll(true)
                .show(ui, |ui| {
                    egui::Grid::new("skill_list_grid")
                        .num_columns(3)
                        .min_row_height(45.0)
                        .striped(true)
                        .show(ui, |ui| {
                            for skill_data in game_data.skills.iter().filter(|skill_data| {
                                if ui_state_debug_skill_list.name_filter.is_empty() {
                                    true
                                } else {
                                    skill_data
                                        .name
                                        .contains(&ui_state_debug_skill_list.name_filter)
                                }
                            }) {
                                if let Some((icon_texture_id, icon_uv)) =
                                    icons.get_skill_icon(skill_data.icon_number as usize)
                                {
                                    ui.add(
                                        egui::Image::new(icon_texture_id, [40.0, 40.0]).uv(icon_uv),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui_add_skill_tooltip(ui, &game_data, skill_data.id);
                                    });
                                } else {
                                    ui.label(" ");
                                }
                                ui.label(format!("{}", skill_data.id.get()));
                                ui.label(&skill_data.name);
                                ui.label(format!("{:?}", skill_data.skill_type));

                                if matches!(app_state.current(), AppState::Game)
                                    && ui.button("Learn").clicked()
                                {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::Chat(format!(
                                                "/skill add {}",
                                                skill_data.id.get()
                                            )))
                                            .ok();
                                    }
                                }

                                ui.end_row();
                            }
                        });
                });
        });
}
