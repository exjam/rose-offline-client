use bevy::{
    ecs::query::WorldQuery,
    input::Input,
    math::Vec3,
    prelude::{
        Camera, Camera3d, Entity, EventWriter, GlobalTransform, MouseButton, Query, Res, ResMut,
        With,
    },
    render::camera::Projection,
    window::Windows,
};
use bevy_egui::EguiContext;
use bevy_rapier3d::prelude::{CollisionGroups, QueryFilter, RapierContext};

use rose_game_common::components::{ItemDrop, Team};

use crate::{
    components::{
        ColliderParent, PlayerCharacter, Position, ZoneObject, COLLISION_FILTER_CLICKABLE,
        COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_PLAYER,
    },
    events::PlayerCommandEvent,
    ray_from_screenspace::ray_from_screenspace,
    resources::SelectedTarget,
};

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    team: &'w Team,
}

#[allow(clippy::too_many_arguments)]
pub fn game_mouse_input_system(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &Projection, &GlobalTransform), With<Camera3d>>,
    rapier_context: Res<RapierContext>,
    mut egui_ctx: ResMut<EguiContext>,
    query_collider_parent: Query<&ColliderParent>,
    query_hit_entity: Query<(
        Option<&Team>,
        Option<&Position>,
        Option<&ItemDrop>,
        Option<&ZoneObject>,
    )>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    mut selected_target: ResMut<SelectedTarget>,
) {
    selected_target.hover = None;

    if egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse is over UI
        return;
    }

    let cursor_position =
        if let Some(cursor_position) = windows.get_primary().and_then(|w| w.cursor_position()) {
            cursor_position
        } else {
            return;
        };

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    for (camera, camera_projection, camera_transform) in query_camera.iter() {
        if let Some((ray_origin, ray_direction)) = ray_from_screenspace(
            cursor_position,
            &windows,
            camera,
            camera_projection,
            camera_transform,
        ) {
            if let Some((collider_entity, distance)) = rapier_context.cast_ray(
                ray_origin,
                ray_direction,
                10000000.0,
                false,
                QueryFilter::new().groups(CollisionGroups::new(
                    COLLISION_FILTER_CLICKABLE,
                    !COLLISION_GROUP_PLAYER & !COLLISION_GROUP_PHYSICS_TOY,
                )),
            ) {
                let hit_position = ray_origin + ray_direction * distance;
                let hit_entity = query_collider_parent
                    .get(collider_entity)
                    .map_or(collider_entity, |collider_parent| collider_parent.entity);

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
                        selected_target.hover = Some(hit_entity);

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
                        selected_target.hover = Some(hit_entity);

                        if mouse_button_input.just_pressed(MouseButton::Left) {
                            if selected_target
                                .selected
                                .map_or(false, |selected_entity| selected_entity == hit_entity)
                            {
                                if hit_team.id == Team::DEFAULT_NPC_TEAM_ID
                                    || hit_team.id == player.team.id
                                {
                                    // Move towards friendly
                                    if let Some(hit_entity_position) = hit_entity_position {
                                        player_command_events.send(PlayerCommandEvent::Move(
                                            hit_entity_position.clone(),
                                            Some(hit_entity),
                                        ));
                                    }
                                } else {
                                    // Attack enemy
                                    player_command_events
                                        .send(PlayerCommandEvent::Attack(hit_entity));
                                }
                            } else {
                                selected_target.selected = Some(hit_entity);
                            }
                        }
                    }
                }
            }
        }
    }
}
