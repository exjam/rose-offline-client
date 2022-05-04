use bevy::{
    core::Time,
    math::{Mat4, Vec2, Vec3},
    prelude::{Camera, EventWriter, GlobalTransform, Parent, Query, Res, Transform, With},
    render::camera::RenderTarget,
    window::{Window, Windows},
};
use bevy_rapier3d::prelude::{InteractionGroups, RapierContext};

use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{
        ColliderParent, CollisionRayCastSource, EventObject, WarpObject,
        COLLISION_FILTER_COLLIDABLE,
    },
    events::QuestTriggerEvent,
    resources::GameConnection,
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
    camera_transform: &GlobalTransform,
) -> Option<(Vec3, Vec3)> {
    let view = camera_transform.compute_matrix();
    let window = get_window_for_camera(windows, camera)?;
    let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
    let projection = camera.projection_matrix;

    // 2D Normalized device coordinate cursor position from (-1, -1) to (1, 1)
    let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
    let ndc_to_world: Mat4 = view * projection.inverse();
    let world_to_ndc = projection * view;
    let is_orthographic = projection.w_axis[3] == 1.0;

    // Compute the cursor position at the near plane. The bevy camera looks at -Z.
    let ndc_near = world_to_ndc.transform_point3(-Vec3::Z * camera.near).z;
    let cursor_pos_near = ndc_to_world.transform_point3(cursor_ndc.extend(ndc_near));

    // Compute the ray's direction depending on the projection used.
    let ray_direction = match is_orthographic {
        true => view.transform_vector3(-Vec3::Z), // All screenspace rays are parallel in ortho
        false => cursor_pos_near - camera_transform.translation, // Direction from camera to cursor
    };

    Some((cursor_pos_near, ray_direction))
}

#[allow(clippy::too_many_arguments)]
pub fn collision_system(
    mut query_entity_ray: Query<(&GlobalTransform, &Parent), With<CollisionRayCastSource>>,
    mut query_parent: Query<&mut Transform>,
    mut query_event_object: Query<&mut EventObject>,
    mut query_warp_object: Query<&mut WarpObject>,
    query_collider_parent: Query<&ColliderParent>,
    mut quest_trigger_events: EventWriter<QuestTriggerEvent>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
    game_connection: Option<Res<GameConnection>>,
) {
    // Cast down to collide entities with ground
    for (transform, parent) in query_entity_ray.iter_mut() {
        let ray_origin = transform.translation;
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);

        if let Some((collider_entity, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            10000000.0,
            false,
            InteractionGroups::all().with_memberships(COLLISION_FILTER_COLLIDABLE),
            None,
        ) {
            if let Ok(hit_entity) = query_collider_parent
                .get(collider_entity)
                .map(|collider_parent| collider_parent.entity)
            {
                if let Ok(mut hit_event_object) = query_event_object.get_mut(hit_entity) {
                    if time.seconds_since_startup() - hit_event_object.last_collision > 5.0 {
                        if !hit_event_object.quest_trigger_name.is_empty() {
                            quest_trigger_events.send(QuestTriggerEvent::DoTrigger(
                                hit_event_object.quest_trigger_name.as_str().into(),
                            ));
                        }

                        hit_event_object.last_collision = time.seconds_since_startup();
                    }

                    continue; // Skip collision
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

                    continue; // Skip collision
                }
            }

            if let Ok(mut parent_transform) = query_parent.get_mut(parent.0) {
                let hit_point = ray_origin + ray_direction * distance;
                parent_transform.translation.y = hit_point.y;
            }
        }
    }
}
