use bevy::{
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, EventReader, EventWriter, Query, Res, State, With},
};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{ActiveMotion, PlayerCharacter},
    events::{GameConnectionEvent, LoadZoneEvent, ZoneEvent},
    free_camera::FreeCamera,
    orbit_camera::OrbitCamera,
    resources::{AppState, GameConnection},
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
            .remove::<ActiveMotion>()
            .insert(OrbitCamera::new(
                player_entity,
                Vec3::new(0.0, 1.7, 0.0),
                15.0,
            ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn game_zone_change_system(
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    mut game_connection_events: EventReader<GameConnectionEvent>,
    mut zone_events: EventReader<ZoneEvent>,
    game_connection: Option<Res<GameConnection>>,
    app_state: Res<State<AppState>>,
) {
    if !matches!(app_state.current(), AppState::Game) {
        // Only run during game app state, its confusing how to
        // combine state + stages so I just do it here
        return;
    }

    for event in game_connection_events.iter() {
        if let &GameConnectionEvent::JoiningZone(zone_id) = event {
            load_zone_events.send(LoadZoneEvent::new(zone_id));
        }
    }

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
