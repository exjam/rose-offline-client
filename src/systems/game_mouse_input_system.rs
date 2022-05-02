use bevy::{
    hierarchy::Parent,
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
use bevy_inspector_egui::egui;
use bevy_rapier3d::prelude::{InteractionGroups, RapierContext};

use rose_game_common::components::{ItemDrop, Team};

use crate::{
    components::{
        ClientEntityName, PlayerCharacter, Position, SelectedTarget, COLLISION_FILTER_CLICKABLE,
    },
    events::PlayerCommandEvent,
    systems::{collision_system::ray_from_screenspace, ZoneObject},
};

#[allow(clippy::too_many_arguments)]
pub fn game_mouse_input_system(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    rapier_context: Res<RapierContext>,
    mut egui_ctx: ResMut<EguiContext>,
    query_parent: Query<&Parent>,
    query_hit_entity: Query<(
        Option<&ClientEntityName>,
        Option<&Team>,
        Option<&Position>,
        Option<&ItemDrop>,
        Option<&ZoneObject>,
    )>,
    query_player: Query<(Entity, &Team, Option<&SelectedTarget>), With<PlayerCharacter>>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
) {
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
        if let Some((ray_origin, ray_direction)) =
            ray_from_screenspace(cursor_position, &windows, camera, camera_transform)
        {
            let hit = rapier_context.cast_ray(
                ray_origin,
                ray_direction,
                10000000.0,
                false,
                InteractionGroups::all().with_memberships(COLLISION_FILTER_CLICKABLE),
                None,
            );

            if let Some((hit_entity, distance)) = hit {
                let hit_position = ray_origin + ray_direction * distance;
                let hit_entity = query_parent.get(hit_entity).map_or(hit_entity, |x| x.0);

                if let Ok((
                    hit_client_entity_name,
                    hit_team,
                    hit_entity_position,
                    hit_item_drop,
                    hit_zone_object,
                )) = query_hit_entity.get(hit_entity)
                {
                    if let Some(hit_client_entity_name) = hit_client_entity_name {
                        egui::show_tooltip(
                            egui_ctx.ctx_mut(),
                            egui::Id::new("entity_mouse_tooltip"),
                            |ui| {
                                ui.label(&hit_client_entity_name.name);
                            },
                        );
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
