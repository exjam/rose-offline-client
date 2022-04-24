use bevy::{
    hierarchy::{BuildChildren, Parent},
    math::Vec3,
    prelude::{
        Assets, Color, Commands, Component, Entity, GlobalTransform, Handle, Query, ResMut,
        Transform, With, Without,
    },
    render::mesh::skinning::SkinnedMesh,
};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};
use bevy_rapier3d::prelude::{ColliderPositionComponent, ColliderShapeComponent};
use rand::prelude::SliceRandom;

use crate::components::{DebugRenderCollider, DebugRenderSkeleton};

#[derive(Component)]
pub struct DebugRenderSkeletonData {
    polyline_entity: Entity,
    polyline_entity_up: Entity,
    polyline: Handle<Polyline>,
    polyline_up: Handle<Polyline>,
}

#[derive(Component)]
pub struct DebugRenderColliderData {
    polyline_entity: Entity,
    polyline: Handle<Polyline>,
}

const COLOR_LIST: [Color; 38] = [
    Color::ALICE_BLUE,
    Color::ANTIQUE_WHITE,
    Color::AQUAMARINE,
    Color::AZURE,
    Color::BEIGE,
    Color::BISQUE,
    Color::BLACK,
    Color::BLUE,
    Color::CRIMSON,
    Color::CYAN,
    Color::DARK_GRAY,
    Color::DARK_GREEN,
    Color::FUCHSIA,
    Color::GOLD,
    Color::GRAY,
    Color::GREEN,
    Color::INDIGO,
    Color::LIME_GREEN,
    Color::MAROON,
    Color::MIDNIGHT_BLUE,
    Color::NAVY,
    Color::NONE,
    Color::OLIVE,
    Color::ORANGE,
    Color::ORANGE_RED,
    Color::PINK,
    Color::PURPLE,
    Color::RED,
    Color::SALMON,
    Color::SEA_GREEN,
    Color::SILVER,
    Color::TEAL,
    Color::TOMATO,
    Color::TURQUOISE,
    Color::VIOLET,
    Color::WHITE,
    Color::YELLOW,
    Color::YELLOW_GREEN,
];

fn generate_cuboid_polyline(vertices: &mut Vec<Vec3>, size: Vec3) {
    // Front face
    vertices.push(Vec3::new(-size.x, -size.y, -size.z));
    vertices.push(Vec3::new(size.x, -size.y, -size.z));
    vertices.push(Vec3::new(size.x, size.y, -size.z));
    vertices.push(Vec3::new(-size.x, size.y, -size.z));
    vertices.push(Vec3::new(-size.x, -size.y, -size.z));
    vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));

    // Back face
    vertices.push(Vec3::new(-size.x, -size.y, size.z));
    vertices.push(Vec3::new(size.x, -size.y, size.z));
    vertices.push(Vec3::new(size.x, size.y, size.z));
    vertices.push(Vec3::new(-size.x, size.y, size.z));
    vertices.push(Vec3::new(-size.x, -size.y, size.z));
    vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));

    // Sides
    vertices.push(Vec3::new(-size.x, -size.y, size.z));
    vertices.push(Vec3::new(-size.x, -size.y, -size.z));
    vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));

    vertices.push(Vec3::new(size.x, -size.y, size.z));
    vertices.push(Vec3::new(size.x, -size.y, -size.z));
    vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));

    vertices.push(Vec3::new(size.x, size.y, size.z));
    vertices.push(Vec3::new(size.x, size.y, -size.z));
    vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));

    vertices.push(Vec3::new(-size.x, size.y, size.z));
    vertices.push(Vec3::new(-size.x, size.y, -size.z));
    vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
}

pub fn debug_render_collider_system(
    mut commands: Commands,
    query_update_collider_shape: Query<
        (
            Entity,
            &GlobalTransform,
            &ColliderShapeComponent,
            &ColliderPositionComponent,
            Option<&DebugRenderColliderData>,
        ),
        With<DebugRenderCollider>,
    >,
    query_remove_debug_collider: Query<
        (Entity, &DebugRenderColliderData),
        Without<DebugRenderCollider>,
    >,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
) {
    let mut rng = rand::thread_rng();

    for (entity, model_transform, collider_shape, collider_position, debug_polyline_data) in
        query_update_collider_shape.iter()
    {
        let inverse_transform = Transform::from_matrix(model_transform.compute_matrix().inverse());

        if let Some(cuboid) = collider_shape.as_cuboid() {
            if let Some(debug_polyline_data) = debug_polyline_data.as_ref() {
                if let Some(polyline) = polylines.get_mut(&debug_polyline_data.polyline) {
                    polyline.vertices.clear();
                    generate_cuboid_polyline(&mut polyline.vertices, cuboid.half_extents.into());
                }

                commands.entity(debug_polyline_data.polyline_entity).insert(
                    inverse_transform
                        * Transform::from_translation(collider_position.translation.into())
                            .with_rotation(collider_position.rotation.into()),
                );
            } else {
                // Add a new debug skeleton
                let mut polyline_vertices = Vec::new();
                generate_cuboid_polyline(&mut polyline_vertices, cuboid.half_extents.into());

                let polyline = polylines.add(Polyline {
                    vertices: polyline_vertices,
                });
                let polyline_entity = commands
                    .spawn_bundle(PolylineBundle {
                        polyline: polyline.clone(),
                        material: polyline_materials.add(PolylineMaterial {
                            width: 2.0,
                            color: *COLOR_LIST.choose(&mut rng).unwrap(),
                            ..Default::default()
                        }),
                        transform: inverse_transform
                            * Transform::from_translation(collider_position.translation.into())
                                .with_rotation(collider_position.rotation.into()),
                        ..Default::default()
                    })
                    .id();

                commands
                    .entity(entity)
                    .insert(DebugRenderColliderData {
                        polyline_entity,
                        polyline,
                    })
                    .add_child(polyline_entity);
            }
        }
    }

    for (entity, debug_collider_data) in query_remove_debug_collider.iter() {
        commands
            .entity(debug_collider_data.polyline_entity)
            .despawn();
        commands.entity(entity).remove::<DebugRenderColliderData>();
    }
}

pub fn debug_render_skeleton_system(
    mut commands: Commands,
    query_update_debug_skeleton: Query<
        (
            Entity,
            &Transform,
            &SkinnedMesh,
            Option<&DebugRenderSkeletonData>,
        ),
        With<DebugRenderSkeleton>,
    >,
    query_remove_debug_skeleton: Query<
        (Entity, &DebugRenderSkeletonData),
        Without<DebugRenderSkeleton>,
    >,
    query_bone: Query<(&GlobalTransform, Option<&Parent>)>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
) {
    let mut rng = rand::thread_rng();

    for (entity, model_transform, skinned_mesh, debug_skeleton) in
        query_update_debug_skeleton.iter()
    {
        let transform = Transform::from_matrix(model_transform.compute_matrix().inverse());

        if let Some(debug_skeleton) = debug_skeleton.as_ref() {
            // Update existing debug skeleton
            if let Some(polyline) = polylines.get_mut(debug_skeleton.polyline.clone()) {
                polyline.vertices.clear();

                for bone_entity in skinned_mesh.joints.iter() {
                    if let Ok((transform, parent)) = query_bone.get(*bone_entity) {
                        if let Some((parent_transform, _)) =
                            parent.and_then(|x| query_bone.get(x.0).ok())
                        {
                            polyline.vertices.push(transform.translation);
                            polyline.vertices.push(parent_transform.translation);
                            polyline
                                .vertices
                                .push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
                        }
                    }
                }
            }
            if let Some(polyline) = polylines.get_mut(debug_skeleton.polyline_up.clone()) {
                polyline.vertices.clear();

                for bone_entity in skinned_mesh.joints.iter() {
                    if let Ok((transform, _)) = query_bone.get(*bone_entity) {
                        polyline.vertices.push(transform.translation);
                        polyline.vertices.push(
                            transform.translation
                                + transform.rotation.mul_vec3(Vec3::new(0.0, 0.2, 0.0)),
                        );
                        polyline
                            .vertices
                            .push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
                    }
                }
            }

            commands
                .entity(debug_skeleton.polyline_entity)
                .insert(transform);
            commands
                .entity(debug_skeleton.polyline_entity_up)
                .insert(transform);
        } else {
            // Add a new debug skeleton
            let mut polyline_vertices = Vec::new();
            let mut polyline_vertices_up = Vec::new();
            for bone_entity in skinned_mesh.joints.iter() {
                if let Ok((transform, parent)) = query_bone.get(*bone_entity) {
                    if let Some((parent_transform, _)) =
                        parent.and_then(|x| query_bone.get(x.0).ok())
                    {
                        polyline_vertices.push(transform.translation);
                        polyline_vertices.push(parent_transform.translation);
                        polyline_vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));

                        polyline_vertices_up.push(transform.translation);
                        polyline_vertices_up.push(
                            transform.translation
                                + transform.rotation.mul_vec3(Vec3::new(0.0, 0.2, 0.0)),
                        );
                        polyline_vertices_up.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
                    }
                }
            }

            let polyline = polylines.add(Polyline {
                vertices: polyline_vertices,
            });
            let polyline_entity = commands
                .spawn_bundle(PolylineBundle {
                    polyline: polyline.clone(),
                    material: polyline_materials.add(PolylineMaterial {
                        width: 4.0,
                        color: *COLOR_LIST.choose(&mut rng).unwrap(),
                        perspective: false,
                        depth_test: false,
                    }),
                    transform,
                    ..Default::default()
                })
                .id();

            let polyline_up = polylines.add(Polyline {
                vertices: polyline_vertices_up,
            });
            let polyline_entity_up = commands
                .spawn_bundle(PolylineBundle {
                    polyline: polyline_up.clone(),
                    material: polyline_materials.add(PolylineMaterial {
                        width: 4.0,
                        color: *COLOR_LIST.choose(&mut rng).unwrap(),
                        perspective: false,
                        depth_test: false,
                    }),
                    transform,
                    ..Default::default()
                })
                .id();

            commands
                .entity(entity)
                .insert(DebugRenderSkeletonData {
                    polyline_entity,
                    polyline_entity_up,
                    polyline,
                    polyline_up,
                })
                .add_child(polyline_entity)
                .add_child(polyline_entity_up);
        }
    }

    for (entity, debug_skeleton) in query_remove_debug_skeleton.iter() {
        commands.entity(debug_skeleton.polyline_entity).despawn();
        commands.entity(debug_skeleton.polyline_entity_up).despawn();
        commands.entity(entity).remove::<DebugRenderSkeletonData>();
    }
}
