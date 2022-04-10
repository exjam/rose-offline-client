use bevy::prelude::{Camera, Query, ResMut, Transform, With};
use bevy_egui::{egui, EguiContext};

use crate::{
    fly_camera::FlyCameraController, follow_camera::FollowCameraController, ui::UiStateDebugWindows,
};

pub fn ui_debug_camera_info_system(
    mut egui_context: ResMut<EguiContext>,
    camera_query: Query<
        (
            &Transform,
            Option<&FollowCameraController>,
            Option<&FlyCameraController>,
        ),
        With<Camera>,
    >,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    egui::Window::new("Camera")
        .open(&mut ui_state_debug_windows.camera_info_open)
        .show(egui_context.ctx_mut(), |ui| {
            let (transform, follow_camera_controller, fly_camera_controller) =
                camera_query.single();

            egui::Grid::new("camera_info_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Translation:");
                    ui.label(format!("{}", transform.translation));
                    ui.end_row();

                    ui.label("Forward:");
                    ui.label(format!("{}", transform.forward()));
                    ui.end_row();

                    if follow_camera_controller.is_some() {
                        ui.label("Type:");
                        ui.label("Follow Camera");
                        ui.end_row();
                    }

                    if fly_camera_controller.is_some() {
                        ui.label("Type:");
                        ui.label("Fly Camera");
                        ui.end_row();
                    }
                });
        });
}
