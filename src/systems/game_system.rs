use bevy::{
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, EventReader, Query, Res, With},
};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    animation::CameraAnimation,
    components::PlayerCharacter,
    events::ZoneEvent,
    resources::GameConnection,
    systems::{FreeCamera, OrbitCamera},
};

pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
) {
    // Reset camera
    let player_entity = query_player.single();
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<CameraAnimation>()
            .insert(OrbitCamera::new(
                player_entity,
                Vec3::new(0.0, 1.7, 0.0),
                15.0,
            ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn game_zone_change_system(
    mut zone_events: EventReader<ZoneEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    for zone_event in zone_events.iter() {
        match zone_event {
            &ZoneEvent::Loaded(_) => {
                // Tell server we are ready to join the zone
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::JoinZoneRequest)
                        .ok();
                }
            }
        }
    }
}
