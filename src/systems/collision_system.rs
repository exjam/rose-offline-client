use bevy::{
    math::{Quat, Vec3},
    prelude::{
        Assets, Changed, Commands, Entity, EventWriter, Or, Query, Res, Time, Transform, With,
    },
};
use bevy_rapier3d::prelude::{Collider, InteractionGroups, QueryFilter, RapierContext};

use rose_game_common::{components::Destination, messages::client::ClientMessage};

use crate::{
    components::{
        ColliderParent, CollisionHeightOnly, CollisionPlayer, EventObject, NextCommand, Position,
        WarpObject, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_MOVEABLE,
        COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_TERRAIN, COLLISION_GROUP_ZONE_WARP_OBJECT,
    },
    events::QuestTriggerEvent,
    resources::{CurrentZone, GameConnection},
    zone_loader::ZoneLoaderAsset,
};

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
            QueryFilter::new().groups(InteractionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                u32::MAX & !COLLISION_GROUP_PHYSICS_TOY,
            )),
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
            QueryFilter::new().groups(InteractionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                u32::MAX & !COLLISION_GROUP_PHYSICS_TOY,
            )),
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
                QueryFilter::new().groups(InteractionGroups::new(
                    COLLISION_FILTER_COLLIDABLE,
                    u32::MAX & !COLLISION_GROUP_ZONE_TERRAIN & !COLLISION_GROUP_PHYSICS_TOY,
                )),
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
            QueryFilter::new().groups(InteractionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                u32::MAX & !COLLISION_GROUP_PHYSICS_TOY,
            )),
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
            QueryFilter::new().groups(
                InteractionGroups::all().with_filter(
                    COLLISION_GROUP_ZONE_EVENT_OBJECT | COLLISION_GROUP_ZONE_WARP_OBJECT,
                ),
            ),
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
