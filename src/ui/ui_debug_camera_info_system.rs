use bevy::prelude::{Camera, Query, ResMut, Transform, With};
use bevy_egui::{egui, EguiContext};

use crate::{free_camera::FreeCamera, orbit_camera::OrbitCamera, ui::UiStateDebugWindows};

pub fn ui_debug_camera_info_system(
    mut egui_context: ResMut<EguiContext>,
    camera_query: Query<(&Transform, Option<&FreeCamera>, Option<&OrbitCamera>), With<Camera>>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    egui::Window::new("Camera")
        .open(&mut ui_state_debug_windows.camera_info_open)
        .show(egui_context.ctx_mut(), |ui| {
            let (transform, free_camera, orbit_camera) = camera_query.single();

            egui::Grid::new("camera_info_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Translation:");
                    ui.label(format!("{}", transform.translation));
                    ui.end_row();

                    ui.label("Forward:");
                    ui.label(format!("{}", transform.forward()));
                    ui.end_row();

                    if free_camera.is_some() {
                        ui.label("Type:");
                        ui.label("Free Camera");
                        ui.end_row();
                    }

                    if orbit_camera.is_some() {
                        ui.label("Type:");
                        ui.label("Orbit Camera");
                        ui.end_row();
                    }
                });
        });
}
