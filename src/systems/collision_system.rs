use bevy::{
    input::Input,
    math::{Mat4, Vec2, Vec3},
    prelude::{
        Assets, Camera, Commands, Entity, EventReader, EventWriter, GlobalTransform, Handle, Local,
        Mesh, MouseButton, Parent, Query, Res, Transform, With, Without,
    },
    render::{
        camera::RenderTarget,
        mesh::{Indices, VertexAttributeValues},
    },
    window::{CursorMoved, Window, Windows},
};
use bevy_rapier3d::{
    physics::{
        ColliderBundle, IntoEntity, QueryPipelineColliderComponentsQuery,
        QueryPipelineColliderComponentsSet,
    },
    prelude::{
        ColliderFlags, ColliderFlagsComponent, ColliderShape, ColliderShapeComponent,
        InteractionGroups, QueryPipeline, Ray,
    },
};

use crate::{
    components::{CollisionRayCastSource, CollisionTriMesh, COLLISION_GROUP_PLAYER_MOVEABLE},
    events::PickingEvent,
};

#[derive(Default)]
pub struct MousePosition {
    position: Vec2,
}

fn get_window_for_camera<'a>(windows: &'a Windows, camera: &Camera) -> Option<&'a Window> {
    match camera.target {
        RenderTarget::Window(window_id) => match windows.get(window_id) {
            None => None,
            window => window,
        },
        _ => None,
    }
}

fn from_screenspace(
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
    query_pipeline: Res<QueryPipeline>,
    colliders: QueryPipelineColliderComponentsQuery,
) {
    // Cast down to collide entities with ground
    let colliders = QueryPipelineColliderComponentsSet(&colliders);

    for (transform, parent) in query_entity_ray.iter_mut() {
        let ray = Ray::new(transform.translation.into(), [0.0, -1.0, 0.0].into());
        let hit = query_pipeline.cast_ray(
            &colliders,
            &ray,
            10000000.0,
            false,
            InteractionGroups::all().with_filter(COLLISION_GROUP_PLAYER_MOVEABLE),
            None,
        );

        if let Some((_, distance)) = hit {
            if let Ok(mut parent_transform) = query_parent.get_mut(parent.0) {
                let hit_point = ray.point_at(distance);
                parent_transform.translation.y = hit_point.y;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collision_picking_system(
    mouse_button_input: Res<Input<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor_position: Local<MousePosition>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform)>,
    query_pipeline: Res<QueryPipeline>,
    colliders: QueryPipelineColliderComponentsQuery,
    mut picking_events: EventWriter<PickingEvent>,
) {
    let colliders = QueryPipelineColliderComponentsSet(&colliders);

    if let Some(event) = cursor_moved_events.iter().last() {
        cursor_position.position = event.position;
    }

    if mouse_button_input.just_pressed(MouseButton::Left) {
        for (camera, camera_transform) in query_camera.iter() {
            if let Some((origin, direction)) =
                from_screenspace(cursor_position.position, &windows, camera, camera_transform)
            {
                let ray = Ray::new(origin.into(), direction.into());
                let hit = query_pipeline.cast_ray(
                    &colliders,
                    &ray,
                    10000000.0,
                    true,
                    InteractionGroups::default(),
                    None,
                );

                if let Some((hit_object, distance)) = hit {
                    picking_events.send(PickingEvent {
                        entity: hit_object.0.entity(),
                        position: ray.point_at(distance).into(),
                    });
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn collision_add_colliders_system(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    query_add_trimesh: Query<
        (Entity, &GlobalTransform, &Handle<Mesh>),
        (With<CollisionTriMesh>, Without<ColliderShapeComponent>),
    >,
) {
    for (entity, global_transform, mesh_handle) in query_add_trimesh.iter() {
        let global_transform_mat = global_transform.compute_matrix();
        if let Some(mesh) = meshes.get(mesh_handle) {
            if let Some(VertexAttributeValues::Float32x3(vertices)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                if let Some(Indices::U16(indices)) = mesh.indices() {
                    let mut rapier_verts = Vec::new();
                    let mut rapier_indices = Vec::new();
                    for vert in vertices.iter() {
                        let pos = global_transform_mat
                            .transform_point3(Vec3::new(vert[0], vert[1], vert[2]));
                        rapier_verts.push([pos.x, pos.y, pos.z].into());
                    }

                    for index in indices.chunks(3) {
                        rapier_indices.push([index[0] as u32, index[2] as u32, index[1] as u32]);
                    }

                    commands.entity(entity).insert_bundle(ColliderBundle {
                        shape: ColliderShapeComponent(ColliderShape::trimesh(
                            rapier_verts,
                            rapier_indices,
                        )),
                        flags: ColliderFlagsComponent(ColliderFlags {
                            collision_groups: InteractionGroups::default(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                }
            }
        }
    }
}
