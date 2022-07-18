use bevy::{
    math::{Mat4, Quat, Vec2, Vec3},
    prelude::{
        Assets, Camera, Changed, Commands, Entity, EventWriter, GlobalTransform, Or, Query, Res,
        Time, Transform, With,
    },
    render::camera::{Projection, RenderTarget},
    window::{Window, Windows},
};
use bevy_rapier3d::prelude::{Collider, InteractionGroups, RapierContext};

use rose_game_common::{components::Destination, messages::client::ClientMessage};

use crate::{
    components::{
        ColliderParent, CollisionHeightOnly, CollisionPlayer, EventObject, NextCommand, Position,
        WarpObject, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_MOVEABLE,
        COLLISION_GROUP_ZONE_EVENT_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
        COLLISION_GROUP_ZONE_WARP_OBJECT,
    },
    events::QuestTriggerEvent,
    resources::{CurrentZone, GameConnection},
    zone_loader::ZoneLoaderAsset,
};

fn get_window_for_camera<'a>(windows: &'a Windows, camera: &Camera) -> Option<&'a Window> {
    match camera.target {
        RenderTarget::Window(window_id) => match windows.get(window_id) {
            None => None,
            window => window,
        },
        _ => None,
    }
}

pub fn ray_from_screenspace(
    cursor_pos_screen: Vec2,
    windows: &Res<Windows>,
    camera: &Camera,
    camera_projection: &Projection,
    camera_transform: &GlobalTransform,
) -> Option<(Vec3, Vec3)> {
    let view = camera_transform.compute_matrix();
    let window = get_window_for_camera(windows, camera)?;
    let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
    let projection = camera.projection_matrix();

    // 2D Normalized device coordinate cursor position from (-1, -1) to (1, 1)
    let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
    let ndc_to_world: Mat4 = view * projection.inverse();
    let world_to_ndc = projection * view;
    let is_orthographic = projection.w_axis[3] == 1.0;

    // Compute the cursor position at the near plane. The bevy camera looks at -Z.
    let camera_near = match camera_projection {
        Projection::Perspective(perspective_projection) => perspective_projection.near,
        Projection::Orthographic(orthographic_projection) => orthographic_projection.near,
    };
    let ndc_near = world_to_ndc.transform_point3(-Vec3::Z * camera_near).z;
    let cursor_pos_near = ndc_to_world.transform_point3(cursor_ndc.extend(ndc_near));

    // Compute the ray's direction depending on the projection used.
    let ray_direction = match is_orthographic {
        true => view.transform_vector3(-Vec3::Z), // All screenspace rays are parallel in ortho
        false => cursor_pos_near - camera_transform.translation, // Direction from camera to cursor
    };

    Some((cursor_pos_near, ray_direction))
}

#[allow(clippy::too_many_arguments)]
pub fn collision_height_only_system(
    mut query_collision_entity: Query<
        (&mut Position, &mut Transform),
        (
            With<CollisionHeightOnly>,
            Or<(Changed<Position>, Changed<Transform>)>,
        ),
    >,
    rapier_context: Res<RapierContext>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    let current_zone = if let Some(current_zone) = current_zone {
        current_zone
    } else {
        return;
    };
    let current_zone_data =
        if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
            current_zone_data
        } else {
            return;
        };

    for (mut position, mut transform) in query_collision_entity.iter_mut() {
        let ray_origin = Vec3::new(position.x / 100.0, 100000.0, -position.y / 100.0);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);

        // Cast ray down to see if we are standing on any objects
        let collision_height = if let Some((_, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            100000000.0,
            false,
            InteractionGroups::all().with_memberships(COLLISION_FILTER_MOVEABLE),
            None,
        ) {
            Some((ray_origin + ray_direction * distance).y)
        } else {
            None
        };

        // We can never be below the heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        // Update entity translation and position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        transform.translation.y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };
        position.z = transform.translation.y * 100.0;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collision_player_system_join_zoin(
    mut query_collision_entity: Query<(&mut Position, &mut Transform), Changed<CollisionPlayer>>,
    rapier_context: Res<RapierContext>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    let current_zone = if let Some(current_zone) = current_zone {
        current_zone
    } else {
        return;
    };
    let current_zone_data =
        if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
            current_zone_data
        } else {
            return;
        };

    for (mut position, mut transform) in query_collision_entity.iter_mut() {
        let ray_origin = Vec3::new(position.x / 100.0, 100000.0, -position.y / 100.0);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);

        // Cast ray down to see if we are standing on any objects
        let collision_height = if let Some((_, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            100000000.0,
            false,
            InteractionGroups::all().with_memberships(COLLISION_FILTER_MOVEABLE),
            None,
        ) {
            Some((ray_origin + ray_direction * distance).y)
        } else {
            None
        };

        // We can never be below the heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        // Update entity translation and position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        transform.translation.y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };
        position.z = transform.translation.y * 100.0;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collision_player_system(
    mut commands: Commands,
    mut query_collision_entity: Query<
        (Entity, &mut Position, &mut Transform),
        With<CollisionPlayer>,
    >,
    mut query_event_object: Query<&mut EventObject>,
    mut quest_trigger_events: EventWriter<QuestTriggerEvent>,
    mut query_warp_object: Query<&mut WarpObject>,
    query_collider_parent: Query<&ColliderParent>,
    current_zone: Option<Res<CurrentZone>>,
    game_connection: Option<Res<GameConnection>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    let current_zone = if let Some(current_zone) = current_zone {
        current_zone
    } else {
        return;
    };
    let current_zone_data =
        if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
            current_zone_data
        } else {
            return;
        };

    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        // Cast ray forward to collide with walls
        let new_translation = Vec3::new(
            position.x / 100.0,
            transform.translation.y,
            -position.y / 100.0,
        );
        let collider_radius = 0.4;
        let translation_delta = new_translation - transform.translation;
        if translation_delta.length() > 0.00001 {
            let cast_origin = transform.translation + Vec3::new(0.0, 1.2, 0.0);
            let cast_direction = translation_delta.normalize();

            if let Some((_, distance)) = rapier_context.cast_shape(
                cast_origin + cast_direction * collider_radius,
                Quat::default(),
                cast_direction,
                &Collider::ball(collider_radius),
                translation_delta.length(),
                InteractionGroups::new(
                    COLLISION_FILTER_COLLIDABLE,
                    u32::MAX & !COLLISION_GROUP_ZONE_TERRAIN,
                ),
                None,
            ) {
                let collision_translation =
                    cast_origin + translation_delta * (distance.toi - 0.1).max(0.0);
                position.x = collision_translation.x * 100.0;
                position.y = -(collision_translation.z * 100.0);
                position.z = collision_translation.y * 100.0;

                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .insert(NextCommand::with_stop());

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::MoveCollision(position.position))
                        .ok();
                }
            }
        }

        // Cast ray down to see if we are standing on any objects
        let fall_distance = time.delta_seconds() * 9.81;
        let ray_origin = Vec3::new(
            position.x / 100.0,
            position.z / 100.0 + 1.35,
            -position.y / 100.0,
        );
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let collision_height = if let Some((_, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            1.35 + fall_distance,
            false,
            InteractionGroups::all().with_memberships(COLLISION_FILTER_MOVEABLE),
            None,
        ) {
            Some((ray_origin + ray_direction * distance).y)
        } else {
            None
        };

        // We can never be below the heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        let target_y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };

        // Update entity translation and position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;

        if transform.translation.y - target_y > fall_distance {
            transform.translation.y -= fall_distance;
        } else {
            transform.translation.y = target_y;
        }

        position.z = transform.translation.y * 100.0;

        // Check if we are now colliding with any warp / event object
        rapier_context.intersections_with_shape(
            Vec3::new(
                position.x / 100.0,
                position.z / 100.0 + 1.0,
                -position.y / 100.0,
            ),
            Quat::default(),
            &Collider::ball(1.0),
            InteractionGroups::all()
                .with_filter(COLLISION_GROUP_ZONE_EVENT_OBJECT | COLLISION_GROUP_ZONE_WARP_OBJECT),
            None,
            |hit_entity| {
                let hit_entity = query_collider_parent
                    .get(hit_entity)
                    .map_or(hit_entity, |collider_parent| collider_parent.entity);

                if let Ok(mut hit_event_object) = query_event_object.get_mut(hit_entity) {
                    if time.seconds_since_startup() - hit_event_object.last_collision > 5.0 {
                        if !hit_event_object.quest_trigger_name.is_empty() {
                            quest_trigger_events.send(QuestTriggerEvent::DoTrigger(
                                hit_event_object.quest_trigger_name.as_str().into(),
                            ));
                        }

                        hit_event_object.last_collision = time.seconds_since_startup();
                    }
                } else if let Ok(mut hit_warp_object) = query_warp_object.get_mut(hit_entity) {
                    if time.seconds_since_startup() - hit_warp_object.last_collision > 5.0 {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::WarpGateRequest(hit_warp_object.warp_id))
                                .ok();
                        }

                        hit_warp_object.last_collision = time.seconds_since_startup();
                    }
                }
                true
            },
        );
    }
}
