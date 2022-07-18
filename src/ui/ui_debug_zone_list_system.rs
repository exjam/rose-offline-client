use bevy::prelude::{EventWriter, Local, Res, ResMut, State};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    events::LoadZoneEvent,
    resources::{AppState, GameConnection, GameData},
    ui::UiStateDebugWindows,
};

pub struct UiDebugZoneListState {
    pub despawn_other_zones: bool,
}

impl Default for UiDebugZoneListState {
    fn default() -> Self {
        Self {
            despawn_other_zones: true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ui_debug_zone_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_zone_list: Local<UiDebugZoneListState>,
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
            if matches!(app_state.current(), AppState::ZoneViewer) {
                ui.checkbox(
                    &mut ui_state_zone_list.despawn_other_zones,
                    "Despawn other zones",
                );
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::initial(50.0).at_least(50.0))
                .column(egui_extras::Size::remainder().at_least(80.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
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
                .body(|mut body| {
                    for zone in game_data.zone_list.iter() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!("{}", zone.id.get()));
                            });

                            row.col(|ui| {
                                ui.label(&zone.name);
                            });

                            row.col(|ui| match app_state.current() {
                                AppState::Game => {
                                    if ui.button("Teleport").clicked() {
                                        if let Some(game_connection) = game_connection.as_ref() {
                                            game_connection
                                                .client_message_tx
                                                .send(ClientMessage::Chat(format!(
                                                    "/mm {}",
                                                    zone.id.get()
                                                )))
                                                .ok();
                                        }
                                    }
                                }
                                AppState::ZoneViewer => {
                                    if ui.button("Load").clicked() {
                                        load_zone_events.send(LoadZoneEvent {
                                            id: zone.id,
                                            despawn_other_zones: ui_state_zone_list
                                                .despawn_other_zones,
                                        });
                                    }
                                }
                                _ => {}
                            });
                        });
                    }
                });
        });
}
