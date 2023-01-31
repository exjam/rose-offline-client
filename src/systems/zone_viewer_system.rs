use bevy::{
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, Query, ResMut, With},
};

use crate::{
    components::ActiveMotion,
    systems::{FreeCamera, OrbitCamera},
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
            .remove::<OrbitCamera>()
            .remove::<ActiveMotion>()
            .insert(FreeCamera::new(
                Vec3::new(5120.0, 50.0, -5120.0),
                -45.0,
                -20.0,
            ));
    }

    // Open relevant debug windows
    ui_state_debug_windows.camera_info_open = true;
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}
