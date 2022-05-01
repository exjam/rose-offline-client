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

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::exact(45.0))
                .column(egui_extras::Size::initial(50.0).at_least(50.0))
                .column(egui_extras::Size::remainder().at_least(80.0))
                .column(egui_extras::Size::initial(100.0).at_least(100.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Icon");
                    });
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Type");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|mut body| {
                    for skill_data in game_data.skills.iter().filter(|skill_data| {
                        if ui_state_debug_skill_list.name_filter.is_empty() {
                            true
                        } else {
                            skill_data
                                .name
                                .contains(&ui_state_debug_skill_list.name_filter)
                        }
                    }) {
                        body.row(45.0, |mut row| {
                            row.col(|ui| {
                                if let Some((icon_texture_id, icon_uv)) =
                                    icons.get_skill_icon(skill_data.icon_number as usize)
                                {
                                    ui.add(
                                        egui::Image::new(icon_texture_id, [40.0, 40.0]).uv(icon_uv),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui_add_skill_tooltip(ui, &game_data, skill_data.id);
                                    });
                                }
                            });

                            row.col(|ui| {
                                ui.label(format!("{}", skill_data.id.get()));
                            });

                            row.col(|ui| {
                                ui.label(&skill_data.name);
                            });

                            row.col(|ui| {
                                ui.label(format!("{:?}", skill_data.skill_type));
                            });

                            row.col(|ui| {
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
                            });
                        });
                    }
                });
        });
}
