use bevy::{
    input::Input,
    math::Vec3,
    prelude::{
        Camera, Commands, Entity, EventWriter, GlobalTransform, MouseButton, Query, Res, ResMut,
        With,
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
use rose_game_common::components::{ItemDrop, Team};

use crate::{
    components::{PlayerCharacter, Position, SelectedTarget, COLLISION_FILTER_CLICKABLE},
    events::PlayerCommandEvent,
    systems::{collision_system::ray_from_screenspace, ZoneObject},
};

#[allow(clippy::too_many_arguments)]
pub fn game_mouse_input_system(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_pipeline: Res<QueryPipeline>,
    colliders: QueryPipelineColliderComponentsQuery,
    mut egui_ctx: ResMut<EguiContext>,
    query_hit_entity: Query<(
        Option<&Team>,
        Option<&Position>,
        Option<&ItemDrop>,
        Option<&ZoneObject>,
    )>,
    query_player: Query<(Entity, &Team, Option<&SelectedTarget>), With<PlayerCharacter>>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
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

                if let Ok((hit_team, hit_entity_position, hit_item_drop, hit_zone_object)) =
                    query_hit_entity.get(hit_entity)
                {
                    if hit_zone_object.is_some() {
                        if mouse_button_input.just_pressed(MouseButton::Left) {
                            player_command_events.send(PlayerCommandEvent::Move(
                                Position::new(Vec3::new(
                                    hit_position.x * 100.0,
                                    -hit_position.z * 100.0,
                                    f32::max(0.0, hit_position.y * 100.0),
                                )),
                                None,
                            ));
                        }
                    } else if hit_item_drop.is_some() {
                        if mouse_button_input.just_pressed(MouseButton::Left) {
                            if let Some(hit_entity_position) = hit_entity_position {
                                // Move to target item drop, once we are close enough the command_system
                                // will send the pickup client message to perform the actual pickup
                                player_command_events.send(PlayerCommandEvent::Move(
                                    hit_entity_position.clone(),
                                    Some(hit_entity),
                                ));
                            }
                        }
                    } else if let Some(hit_team) = hit_team {
                        if mouse_button_input.just_pressed(MouseButton::Left) {
                            if player_selected_target
                                .map_or(false, |target| target.entity == hit_entity)
                            {
                                if hit_team.id == Team::DEFAULT_NPC_TEAM_ID {
                                    if let Some(hit_entity_position) = hit_entity_position {
                                        player_command_events.send(PlayerCommandEvent::Move(
                                            hit_entity_position.clone(),
                                            Some(hit_entity),
                                        ));
                                    }
                                } else if hit_team.id != player_team.id {
                                    player_command_events
                                        .send(PlayerCommandEvent::Attack(hit_entity));
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
