use bevy::{
    math::Vec3,
    prelude::{Camera, Commands, Entity, PerspectiveCameraBundle, Query, ResMut, Transform, With},
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};

use crate::{
    components::ActiveMotion,
    fly_camera::{FlyCameraBundle, FlyCameraController},
    follow_camera::FollowCameraController,
    ui::UiStateDebugWindows,
};

pub fn zone_viewer_setup_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FollowCameraController>()
            .remove::<ActiveMotion>()
            .insert_bundle(FlyCameraBundle::new(
                FlyCameraController::default(),
                PerspectiveCameraBundle::default(),
                Vec3::new(5120.0, 50.0, -5120.0),
                Vec3::new(5200.0, 0.0, -5200.0),
            ));
    }

    // Open zone list debug window
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}

#[allow(clippy::too_many_arguments)]
pub fn zone_viewer_system(
    mut egui_context: ResMut<EguiContext>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    egui::Window::new("Camera").show(egui_context.ctx_mut(), |ui| {
        let transform = camera_query.single();
        ui.label(format!("Translation: {}", transform.translation));
        ui.label(format!("Forward: {}", transform.forward()));
    });
}
