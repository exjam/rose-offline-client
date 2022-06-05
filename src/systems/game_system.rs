use bevy::{
    math::Vec3,
    prelude::{
        Camera3d, Camera3dBundle, Commands, Entity, EventReader, EventWriter, Query, Res, State,
        Transform, With,
    },
};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{ActiveMotion, PlayerCharacter},
    events::{GameConnectionEvent, LoadZoneEvent, ZoneEvent},
    fly_camera::FlyCameraController,
    follow_camera::{FollowCameraBundle, FollowCameraController},
    resources::{AppState, GameConnection},
};

pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    query_player: Query<(Entity, &Transform), With<PlayerCharacter>>,
) {
    // Reset camera
    let (player_entity, player_transform) = query_player.single();
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FlyCameraController>()
            .remove::<ActiveMotion>()
            .insert_bundle(FollowCameraBundle::new(
                FollowCameraController {
                    follow_entity: Some(player_entity),
                    follow_offset: Vec3::new(0.0, 1.7, 0.0),
                    follow_distance: 15.0,
                    ..Default::default()
                },
                Camera3dBundle::default(),
                player_transform.translation + Vec3::new(10.0, 10.0, 10.0),
                player_transform.translation,
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
