use bevy::{
    math::Vec3,
    prelude::{Camera, Commands, Entity, PerspectiveCameraBundle, Query, Transform, With},
};
use bevy_mod_picking::PickingCameraBundle;

use crate::{bevy_flycam::FlyCam, components::PlayerCharacter};

pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera>>,
    query_player: Query<&Transform, With<PlayerCharacter>>,
) {
    // Remove any other cameras
    for entity in query_cameras.iter() {
        commands.entity(entity).despawn();
    }

    // TODO: Follow PlayerCharacter camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(5100.0, 10.0, -4700.0)
                .looking_at(query_player.single().translation, Vec3::Y),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .insert(FlyCam);
}
