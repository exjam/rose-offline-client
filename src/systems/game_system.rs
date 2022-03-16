use bevy::{
    math::Vec3,
    prelude::{
        Commands, Entity, EventReader, PerspectiveCameraBundle, Query, Res, ResMut, Transform, With,
    },
    render::camera::Camera3d,
};
use bevy_egui::EguiContext;
use rose_game_common::messages::client::{ClientMessage, Move};

use crate::{
    components::PlayerCharacter,
    events::PickingEvent,
    fly_camera::FlyCameraController,
    follow_camera::{FollowCameraBundle, FollowCameraController},
    resources::GameConnection,
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
            .insert_bundle(FollowCameraBundle::new(
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
}

pub fn game_player_move_system(
    mut picking_events: EventReader<PickingEvent>,
    game_connection: Option<Res<GameConnection>>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    if let Some(event) = picking_events.iter().last() {
        if egui_ctx.ctx_mut().wants_pointer_input() || egui_ctx.ctx_mut().wants_keyboard_input() {
            return;
        }

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
