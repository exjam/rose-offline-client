use bevy::{
    input::Input,
    math::Vec3,
    prelude::{Commands, Entity, EventWriter, KeyCode, Local, Query, Res, ResMut, With},
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::PlayerCharacter,
    events::DebugInspectorEvent,
    fly_camera::FlyCameraController,
    follow_camera::FollowCameraController,
    resources::{GameConnection, GameData},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugCameraType {
    Follow,
    Free,
}

pub struct GameDebugUiState {
    show_debug_ui: bool,
    show_zone_list: bool,
    show_object_inspector: bool,
    selected_camera_type: DebugCameraType,
}

impl Default for GameDebugUiState {
    fn default() -> Self {
        Self {
            show_debug_ui: true,
            show_zone_list: false,
            show_object_inspector: false,
            selected_camera_type: DebugCameraType::Follow,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn game_debug_ui_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<GameDebugUiState>,
    query_cameras: Query<Entity, With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    keyboard: Res<Input<KeyCode>>,
    mut debug_inspector_events: EventWriter<DebugInspectorEvent>,
) {
    if keyboard.pressed(KeyCode::LControl) && keyboard.just_pressed(KeyCode::D) {
        ui_state.show_debug_ui = !ui_state.show_debug_ui;
    }

    if !ui_state.show_debug_ui {
        return;
    }

    let ctx = &*egui_context.ctx_mut();
    egui::TopBottomPanel::top("game_debug_ui_top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Camera", |ui| {
                let previous_camera_type = ui_state.selected_camera_type;

                ui.selectable_value(
                    &mut ui_state.selected_camera_type,
                    DebugCameraType::Follow,
                    "Follow",
                );

                ui.selectable_value(
                    &mut ui_state.selected_camera_type,
                    DebugCameraType::Free,
                    "Free",
                );

                if ui_state.selected_camera_type != previous_camera_type {
                    for camera_entity in query_cameras.iter() {
                        match ui_state.selected_camera_type {
                            DebugCameraType::Follow => {
                                commands
                                    .entity(camera_entity)
                                    .remove::<FlyCameraController>()
                                    .insert(FollowCameraController {
                                        follow_entity: query_player.get_single().ok(),
                                        follow_offset: Vec3::new(0.0, 1.7, 0.0),
                                        ..Default::default()
                                    });
                            }
                            DebugCameraType::Free => {
                                commands
                                    .entity(camera_entity)
                                    .remove::<FollowCameraController>()
                                    .insert(FlyCameraController::default());
                            }
                        }
                    }
                }
            });

            ui.menu_button("Cheats", |ui| {
                if ui.button("Move Speed 4000").clicked() {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Chat("/speed 4000".to_string()))
                            .ok();
                    }
                }
            });

            ui.menu_button("View", |ui| {
                ui.selectable_value(&mut ui_state.show_zone_list, true, "Zone List");
                if ui
                    .selectable_label(ui_state.show_object_inspector, "Object Inspector")
                    .clicked()
                {
                    ui_state.show_object_inspector = !ui_state.show_object_inspector;
                    if ui_state.show_object_inspector {
                        debug_inspector_events.send(DebugInspectorEvent::Show);
                        debug_inspector_events
                            .send(DebugInspectorEvent::InspectEntity(query_player.single()));
                    } else {
                        debug_inspector_events.send(DebugInspectorEvent::Hide);
                    }
                }
            });
        });
    });

    let show_zone_list = &mut (*ui_state).show_zone_list;

    egui::Window::new("Zone List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(show_zone_list)
        .show(ctx, |ui| {
            egui::Grid::new("zone_list_grid").show(ui, |ui| {
                ui.label("id");
                ui.label("name");
                ui.end_row();

                for zone in game_data.zone_list.iter() {
                    ui.label(format!("{}", zone.id.get()));
                    ui.label(&zone.name);
                    if ui.button("Teleport").clicked() {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::Chat(format!("/mm {}", zone.id.get())))
                                .ok();
                        }
                    }
                    ui.end_row();
                }
            });
        });
}
