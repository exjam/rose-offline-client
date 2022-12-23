use bevy::prelude::{EventWriter, Local, Res, ResMut, State};
use bevy_egui::{egui, EguiContext};
use regex::Regex;

use rose_data::ZoneId;
use rose_game_common::messages::client::ClientMessage;

use crate::{
    events::LoadZoneEvent,
    resources::{AppState, GameConnection, GameData},
    ui::UiStateDebugWindows,
};

pub struct UiDebugZoneListState {
    despawn_other_zones: bool,
    filter_name: String,
    filtered_zones: Vec<ZoneId>,
}

impl Default for UiDebugZoneListState {
    fn default() -> Self {
        Self {
            despawn_other_zones: true,
            filter_name: String::default(),
            filtered_zones: Vec::default(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ui_debug_zone_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiDebugZoneListState>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Zone List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.zone_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut filter_changed = false;

            egui::Grid::new("zone_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Zone Name Filter:");
                    if ui.text_edit_singleline(&mut ui_state.filter_name).changed() {
                        filter_changed = true;
                    }
                    ui.end_row();

                    if matches!(app_state.current(), AppState::ZoneViewer) {
                        ui.label("Despawn other zones:");
                        ui.checkbox(&mut ui_state.despawn_other_zones, "Despawn");
                        ui.end_row();
                    }
                });

            if ui_state.filter_name.is_empty() && ui_state.filtered_zones.is_empty() {
                filter_changed = true;
            }

            if filter_changed {
                let filter_name_re = if !ui_state.filter_name.is_empty() {
                    Some(
                        Regex::new(&format!("(?i){}", regex::escape(&ui_state.filter_name)))
                            .unwrap(),
                    )
                } else {
                    None
                };

                ui_state.filtered_zones = game_data
                    .zone_list
                    .iter()
                    .filter_map(|zone_data| {
                        if !filter_name_re
                            .as_ref()
                            .map_or(true, |re| re.is_match(zone_data.name))
                        {
                            None
                        } else {
                            Some(zone_data.id)
                        }
                    })
                    .collect();
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::initial(50.0).at_least(50.0))
                .column(egui_extras::Column::remainder().at_least(80.0))
                .column(egui_extras::Column::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(20.0, ui_state.filtered_zones.len(), |row_index, mut row| {
                        if let Some(zone_data) = ui_state
                            .filtered_zones
                            .get(row_index)
                            .and_then(|id| game_data.zone_list.get_zone(*id))
                        {
                            row.col(|ui| {
                                ui.label(format!("{}", zone_data.id.get()));
                            });

                            row.col(|ui| {
                                ui.label(zone_data.name);
                            });

                            row.col(|ui| match app_state.current() {
                                AppState::Game => {
                                    if ui.button("Teleport").clicked() {
                                        if let Some(game_connection) = game_connection.as_ref() {
                                            game_connection
                                                .client_message_tx
                                                .send(ClientMessage::Chat(format!(
                                                    "/mm {}",
                                                    zone_data.id.get()
                                                )))
                                                .ok();
                                        }
                                    }
                                }
                                AppState::ZoneViewer => {
                                    if ui.button("Load").clicked() {
                                        load_zone_events.send(LoadZoneEvent {
                                            id: zone_data.id,
                                            despawn_other_zones: ui_state.despawn_other_zones,
                                        });
                                    }
                                }
                                _ => {}
                            });
                        }
                    });
                });
        });
}
