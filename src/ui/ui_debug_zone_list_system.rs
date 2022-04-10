use bevy::prelude::{EventWriter, Res, ResMut, State};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    events::LoadZoneEvent,
    resources::{AppState, GameConnection, GameData},
    ui::UiStateDebugWindows,
};

#[allow(clippy::too_many_arguments)]
pub fn ui_debug_zone_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    let ctx = egui_context.ctx_mut();
    egui::Window::new("Zone List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.zone_list_open)
        .show(ctx, |ui| {
            egui::Grid::new("zone_list_grid")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("id");
                    ui.label("name");
                    ui.end_row();

                    for zone in game_data.zone_list.iter() {
                        ui.label(format!("{}", zone.id.get()));
                        ui.label(&zone.name);

                        match app_state.current() {
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
                                    load_zone_events.send(LoadZoneEvent::new(zone.id));
                                }
                            }
                            _ => {}
                        }

                        ui.end_row();
                    }
                });
        });
}
