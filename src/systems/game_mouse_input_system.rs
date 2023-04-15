use bevy::{
    ecs::query::WorldQuery,
    input::Input,
    math::Vec3,
    prelude::{
        Camera, Camera3d, Entity, EventWriter, GlobalTransform, MouseButton, Query, Res, ResMut,
        With,
    },
    render::camera::Projection,
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::{CollisionGroups, QueryFilter, RapierContext};

use rose_game_common::components::{ItemDrop, Team};

use crate::{
    components::{
        ClientEntity, ClientEntityType, ColliderParent, PlayerCharacter, Position, ZoneObject,
        COLLISION_FILTER_CLICKABLE, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_PLAYER,
    },
    events::{MoveDestinationEffectEvent, PlayerCommandEvent},
    ray_from_screenspace::ray_from_screenspace,
    resources::{SelectedTarget, UiCursorType, UiResources},
};

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    team: &'w Team,
}

#[allow(clippy::too_many_arguments)]
pub fn game_mouse_input_system(
    mouse_button_input: Res<Input<MouseButton>>,
    mut query_window: Query<&mut Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &Projection, &GlobalTransform), With<Camera3d>>,
    rapier_context: Res<RapierContext>,
    mut egui_ctx: EguiContexts,
    query_collider_parent: Query<&ColliderParent>,
    query_hit_entity: Query<(
        Option<&Team>,
        Option<&Position>,
        Option<&ItemDrop>,
        Option<&ZoneObject>,
        Option<&ClientEntity>,
    )>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    mut move_destination_effect_events: EventWriter<MoveDestinationEffectEvent>,
    mut selected_target: ResMut<SelectedTarget>,
    ui_resources: Res<UiResources>,
) {
    selected_target.hover = None;

    let Ok(mut window) = query_window.get_single_mut() else {
        return;
    };

    if !matches!(window.cursor.grab_mode, CursorGrabMode::None) {
        // Cursor is currently grabbed
        return;
    }

    let Some(cursor_position) = window.cursor_position() else {
        // Failed to get cursor position
        return;
    };

    if !egui_ctx.ctx_mut().wants_pointer_input() {
        let mut cursor_type = UiCursorType::Default;
        let player = if let Ok(player) = query_player.get_single() {
            player
        } else {
            return;
        };
        let Ok((camera, camera_projection, camera_transform)) = query_camera.get_single() else {
            return;
        };

        if let Some((ray_origin, ray_direction)) = ray_from_screenspace(
            cursor_position,
            &window,
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

                if let Ok((
                    hit_team,
                    hit_entity_position,
                    hit_item_drop,
                    hit_zone_object,
                    hit_client_entity,
                )) = query_hit_entity.get(hit_entity)
                {
                    if let Some(hit_client_entity) = hit_client_entity {
                        match hit_client_entity.entity_type {
                            ClientEntityType::Character => cursor_type = UiCursorType::User,
                            ClientEntityType::Monster => cursor_type = UiCursorType::Attack,
                            ClientEntityType::Npc => cursor_type = UiCursorType::Npc,
                            ClientEntityType::ItemDrop => cursor_type = UiCursorType::PickupItem,
                        }
                    }

                    if let Some(hit_team) = hit_team.as_ref() {
                        if hit_team.id != Team::DEFAULT_NPC_TEAM_ID && hit_team.id != player.team.id
                        {
                            cursor_type = UiCursorType::Attack;
                        }
                    }

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

                            move_destination_effect_events.send(MoveDestinationEffectEvent::Show {
                                position: hit_position,
                            });
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

        let cursor = &ui_resources.cursors[cursor_type];
        if window.cursor.icon != cursor.cursor {
            window.cursor.icon = cursor.cursor.clone();
        }
    }
}
