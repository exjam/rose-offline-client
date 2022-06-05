use bevy::{
    math::Vec3,
    prelude::{Camera3d, Camera3dBundle, Commands, Entity, Query, ResMut, With},
};

use crate::{
    components::ActiveMotion,
    fly_camera::{FlyCameraBundle, FlyCameraController},
    follow_camera::FollowCameraController,
    ui::UiStateDebugWindows,
};

pub fn zone_viewer_enter_system(
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
                Camera3dBundle::default(),
                Vec3::new(5120.0, 50.0, -5120.0),
                Vec3::new(5200.0, 0.0, -5200.0),
            ));
    }

    // Open relevant debug windows
    ui_state_debug_windows.camera_info_open = true;
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}
