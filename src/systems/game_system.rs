use bevy::{
    math::Vec3,
    prelude::{
        Camera, Commands, Entity, EventReader, PerspectiveCameraBundle, PerspectiveProjection,
        Query, Res, Transform, With,
    },
};
use rose_game_common::messages::client::{ClientMessage, Move};

use crate::{
    components::PlayerCharacter,
    events::PickingEvent,
    follow_camera::{FollowCameraBundle, FollowCameraController},
    resources::GameConnection,
};

pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, (With<Camera>, With<PerspectiveProjection>)>,
    query_player: Query<(Entity, &Transform), With<PlayerCharacter>>,
) {
    // Remove any other cameras
    for entity in query_cameras.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn camera which follows player
    let (player_entity, player_transform) = query_player.single();
    commands.spawn_bundle(FollowCameraBundle::new(
        FollowCameraController {
            follow_entity: Some(player_entity),
            follow_offset: Vec3::new(0.0, 1.7, 0.0),
            ..Default::default()
        },
        PerspectiveCameraBundle::default(),
        player_transform.translation + Vec3::new(10.0, 10.0, 10.0),
        player_transform.translation,
    ));
}

pub fn game_player_move_system(
    mut picking_events: EventReader<PickingEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    if let Some(event) = picking_events.iter().last() {
        if let Some(game_connection) = game_connection.as_ref() {
            game_connection
                .client_message_tx
                .send(ClientMessage::Move(Move {
                    target_entity_id: None,
                    x: event.position.x * 100.0,
                    y: -event.position.z * 100.0,
                    z: f32::max(0.0, event.position.y * 100.0) as u16,
                }))
                .ok();
        }
    }
}
