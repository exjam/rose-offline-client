use bevy::{
    math::Vec3,
    pbr::StandardMaterial,
    prelude::{
        shape, AssetServer, Assets, Camera3d, Color, Commands, ComputedVisibility, Entity,
        GlobalTransform, Mesh, Query, Res, ResMut, Transform, Visibility, With,
    },
};

use crate::{
    audio::SpatialSound, components::ActiveMotion, free_camera::FreeCamera,
    orbit_camera::OrbitCamera, ui::UiStateDebugWindows,
};

pub fn zone_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    commands.spawn_bundle((
        SpatialSound::new_repeating(asset_server.load("Sound/avata/Cart_Move.wav")),
        Transform::from_translation(Vec3::new(5200.0, 5.0, -5200.0)),
        GlobalTransform::default(),
        meshes.add(
            shape::Icosphere {
                radius: 0.5,
                ..Default::default()
            }
            .into(),
        ),
        materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.0, 1.0),
            ..Default::default()
        }),
        Visibility::default(),
        ComputedVisibility::default(),
    ));
}
