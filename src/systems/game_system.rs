use bevy::{
    input::Input,
    math::Vec3,
    prelude::{
        Camera, Commands, Entity, GlobalTransform, MouseButton, PerspectiveCameraBundle, Query,
        Res, ResMut, Transform, With,
    },
    render::camera::Camera3d,
    window::Windows,
};
use bevy_egui::EguiContext;
use bevy_rapier3d::{
    physics::{QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet},
    prelude::{InteractionGroups, QueryPipeline},
};
use rose_game_common::messages::client::{ClientMessage, Move};

use crate::{
    components::{PlayerCharacter, COLLISION_FILTER_CLICKABLE},
    fly_camera::FlyCameraController,
    follow_camera::{FollowCameraBundle, FollowCameraController},
    resources::GameConnection,
};

use super::{collision_system::ray_from_screenspace, debug_inspector_system::DebugInspectorState};

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

#[allow(clippy::too_many_arguments)]
pub fn game_input_system(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_pipeline: Res<QueryPipeline>,
    colliders: QueryPipelineColliderComponentsQuery,
    mut egui_ctx: ResMut<EguiContext>,
    game_connection: Option<Res<GameConnection>>,
    debug_inspector_state: Res<DebugInspectorState>,
) {
    let colliders = QueryPipelineColliderComponentsSet(&colliders);
    let cursor_position = windows.primary().cursor_position();
    if cursor_position.is_none() {
        // Mouse not in window
        return;
    }
    let cursor_position = cursor_position.unwrap();

    if egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse is over UI
        return;
    }

    if debug_inspector_state.enable_picking {
        // We are currently picking in debug inspector
        return;
    }

    if mouse_button_input.just_pressed(MouseButton::Left) {
        for (camera, camera_transform) in query_camera.iter() {
            if let Some(ray) =
                ray_from_screenspace(cursor_position, &windows, camera, camera_transform)
            {
                let hit = query_pipeline.cast_ray(
                    &colliders,
                    &ray,
                    10000000.0,
                    false,
                    InteractionGroups::all().with_memberships(COLLISION_FILTER_CLICKABLE),
                    None,
                );

                if let Some((_, distance)) = hit {
                    let hit_position = ray.point_at(distance);

                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Move(Move {
                                target_entity_id: None,
                                x: hit_position.x * 100.0,
                                y: -hit_position.z * 100.0,
                                z: f32::max(0.0, hit_position.y * 100.0) as u16,
                            }))
                            .ok();
                    }
                }
            }
        }
    }
}
