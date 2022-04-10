use bevy::{
    input::Input,
    math::Vec3,
    prelude::{Commands, Entity, EventWriter, KeyCode, Local, Query, Res, ResMut, With},
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::PlayerCharacter, events::DebugInspectorEvent, fly_camera::FlyCameraController,
    follow_camera::FollowCameraController, resources::GameConnection,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugCameraType {
    Follow,
    Free,
}

impl Default for DebugCameraType {
    fn default() -> Self {
        Self::Follow
    }
}

#[derive(Default)]
pub struct UiStateDebugWindows {
    pub debug_ui_open: bool,

    pub object_inspector_open: bool,
    pub zone_list_open: bool,
}

#[derive(Default)]
pub struct UiStateDebugMenu {
    selected_camera_type: DebugCameraType,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_debug_menu_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_menu: Local<UiStateDebugMenu>,
    query_cameras: Query<Entity, With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    game_connection: Option<Res<GameConnection>>,
    keyboard: Res<Input<KeyCode>>,
    mut debug_inspector_events: EventWriter<DebugInspectorEvent>,
) {
    if keyboard.pressed(KeyCode::LControl) && keyboard.just_pressed(KeyCode::D) {
        ui_state_debug_windows.debug_ui_open = !ui_state_debug_windows.debug_ui_open;
    }

    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    let ctx = &*egui_context.ctx_mut();
    egui::TopBottomPanel::top("game_debug_ui_top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Camera", |ui| {
                let previous_camera_type = ui_state_debug_menu.selected_camera_type;

                ui.selectable_value(
                    &mut ui_state_debug_menu.selected_camera_type,
                    DebugCameraType::Follow,
                    "Follow",
                );

                ui.selectable_value(
                    &mut ui_state_debug_menu.selected_camera_type,
                    DebugCameraType::Free,
                    "Free",
                );

                if ui_state_debug_menu.selected_camera_type != previous_camera_type {
                    for camera_entity in query_cameras.iter() {
                        match ui_state_debug_menu.selected_camera_type {
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
                ui.selectable_value(
                    &mut ui_state_debug_windows.zone_list_open,
                    true,
                    "Zone List",
                );

                if ui
                    .selectable_label(
                        ui_state_debug_windows.object_inspector_open,
                        "Object Inspector",
                    )
                    .clicked()
                {
                    ui_state_debug_windows.object_inspector_open =
                        !ui_state_debug_windows.object_inspector_open;
                    if ui_state_debug_windows.object_inspector_open {
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
}
