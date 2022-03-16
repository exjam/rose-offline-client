use bevy::{
    input::Input,
    math::Vec3,
    prelude::{Commands, Entity, KeyCode, Local, Query, Res, ResMut, With},
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};

use crate::{
    components::PlayerCharacter, fly_camera::FlyCameraController,
    follow_camera::FollowCameraController,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugCameraType {
    Follow,
    Free,
}

pub struct GameDebugUiState {
    show_debug_ui: bool,
    selected_camera_type: DebugCameraType,
}

impl Default for GameDebugUiState {
    fn default() -> Self {
        Self {
            show_debug_ui: true,
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
    keyboard: Res<Input<KeyCode>>,
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
            })
        });
    });
}
