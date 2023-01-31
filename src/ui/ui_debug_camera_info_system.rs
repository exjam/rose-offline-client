use bevy::prelude::{Camera, Query, ResMut, Transform, With};
use bevy_egui::{egui, EguiContext};
use dolly::prelude::{Arm, YawPitch};

use crate::{
    systems::{FreeCamera, OrbitCamera},
    ui::UiStateDebugWindows,
};

pub fn ui_debug_camera_info_system(
    mut egui_context: ResMut<EguiContext>,
    mut camera_query: Query<
        (
            &Transform,
            Option<&mut FreeCamera>,
            Option<&mut OrbitCamera>,
        ),
        With<Camera>,
    >,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    egui::Window::new("Camera")
        .open(&mut ui_state_debug_windows.camera_info_open)
        .show(egui_context.ctx_mut(), |ui| {
            let (transform, free_camera, orbit_camera) = camera_query.single_mut();

            egui::Grid::new("camera_info_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Translation:");
                    ui.label(format!("{}", transform.translation));
                    ui.end_row();

                    if let Some(mut free_camera) = free_camera {
                        ui.label("Type:");
                        ui.label("Free Camera");
                        ui.end_row();

                        let yaw_pitch = free_camera.rig.driver_mut::<YawPitch>();
                        ui.label("Yaw:");
                        ui.label(format!("{}", yaw_pitch.yaw_degrees));
                        ui.end_row();

                        ui.label("Pitch:");
                        ui.label(format!("{}", yaw_pitch.pitch_degrees));
                        ui.end_row();
                    }

                    if let Some(mut orbit_camera) = orbit_camera {
                        ui.label("Type:");
                        ui.label("Orbit Camera");
                        ui.end_row();

                        let yaw_pitch = orbit_camera.rig.driver_mut::<YawPitch>();
                        ui.label("Yaw:");
                        ui.label(format!("{}", yaw_pitch.yaw_degrees));
                        ui.end_row();

                        ui.label("Pitch:");
                        ui.label(format!("{}", yaw_pitch.pitch_degrees));
                        ui.end_row();

                        let arm_offset_z = orbit_camera.rig.driver_mut::<Arm>().offset.z;
                        ui.label("Arm Offset Z:");
                        ui.label(format!("{}", arm_offset_z));
                        ui.end_row();
                    }
                });
        });
}
