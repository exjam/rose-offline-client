use bevy::{
    input::Input,
    math::Vec3,
    prelude::{
        Camera, Commands, Entity, EventReader, EventWriter, GlobalTransform, MouseButton,
        PerspectiveCameraBundle, Query, Res, ResMut, Transform, With,
    },
    render::camera::Camera3d,
    window::Windows,
};
use bevy_egui::EguiContext;
use bevy_rapier3d::{
    physics::{
        IntoEntity, QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
    },
    prelude::{InteractionGroups, QueryPipeline},
};
use rose_game_common::{
    components::{ItemDrop, Team},
    messages::client::{Attack, ClientMessage, Move},
};

use crate::{
    components::{
        ActiveMotion, ClientEntity, PlayerCharacter, Position, SelectedTarget,
        COLLISION_FILTER_CLICKABLE,
    },
    events::{GameConnectionEvent, LoadZoneEvent, ZoneEvent},
    fly_camera::FlyCameraController,
    follow_camera::{FollowCameraBundle, FollowCameraController},
    resources::GameConnection,
};

use super::{
    collision_system::ray_from_screenspace, debug_inspector_system::DebugInspectorState, ZoneObject,
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
                    ..Default::default()
                },
                PerspectiveCameraBundle::default(),
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
) {
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

#[allow(clippy::too_many_arguments)]
pub fn game_input_system(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_pipeline: Res<QueryPipeline>,
    colliders: QueryPipelineColliderComponentsQuery,
    mut egui_ctx: ResMut<EguiContext>,
    game_connection: Option<Res<GameConnection>>,
    debug_inspector_state: Res<DebugInspectorState>,
    query_hit_entity: Query<(
        Option<&ClientEntity>,
        Option<&Team>,
        Option<&Position>,
        Option<&ItemDrop>,
        Option<&ZoneObject>,
    )>,
    query_player: Query<(Entity, &Team, Option<&SelectedTarget>), With<PlayerCharacter>>,
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

    let (player_entity, player_team, player_selected_target) = query_player.single();

    for (camera, camera_transform) in query_camera.iter() {
        if let Some(ray) = ray_from_screenspace(cursor_position, &windows, camera, camera_transform)
        {
            let hit = query_pipeline.cast_ray(
                &colliders,
                &ray,
                10000000.0,
                false,
                InteractionGroups::all().with_memberships(COLLISION_FILTER_CLICKABLE),
                None,
            );

            if let Some((hit_collider_handle, distance)) = hit {
                let hit_position = ray.point_at(distance);
                let hit_entity = hit_collider_handle.entity();

                if let Ok((
                    hit_client_entity,
                    hit_team,
                    hit_entity_position,
                    hit_item_drop,
                    hit_zone_object,
                )) = query_hit_entity.get(hit_entity)
                {
                    if hit_zone_object.is_some() {
                        if mouse_button_input.just_pressed(MouseButton::Left) {
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
                    } else if let (Some(hit_client_entity), Some(_)) =
                        (hit_client_entity, hit_item_drop)
                    {
                        if mouse_button_input.just_pressed(MouseButton::Left) {
                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::PickupItemDrop(hit_client_entity.id))
                                    .ok();
                            }
                        }
                    } else if let (Some(hit_client_entity), Some(hit_team)) =
                        (hit_client_entity, hit_team)
                    {
                        if mouse_button_input.just_pressed(MouseButton::Left) {
                            if player_selected_target
                                .map_or(false, |target| target.entity == hit_entity)
                            {
                                if hit_team.id == Team::DEFAULT_NPC_TEAM_ID {
                                    if let Some(hit_entity_position) = hit_entity_position {
                                        if let Some(game_connection) = game_connection.as_ref() {
                                            game_connection
                                                .client_message_tx
                                                .send(ClientMessage::Move(Move {
                                                    target_entity_id: Some(hit_client_entity.id),
                                                    x: hit_entity_position.position.x,
                                                    y: hit_entity_position.position.y,
                                                    z: hit_entity_position.position.z as u16,
                                                }))
                                                .ok();
                                        }
                                    }
                                } else if hit_team.id != player_team.id {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::Attack(Attack {
                                                target_entity_id: hit_client_entity.id,
                                            }))
                                            .ok();
                                    }
                                }
                            } else {
                                commands
                                    .entity(player_entity)
                                    .insert(SelectedTarget::new(hit_entity));
                            }
                        }
                    }
                }
            }
        }
    }
}
