use bevy::{
    hierarchy::{BuildChildren, Parent},
    math::Vec3,
    prelude::{
        Assets, Color, Commands, Component, Entity, GlobalTransform, Handle, Query, ResMut,
        Transform, With, Without,
    },
};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};

use crate::components::{DebugModelSkeleton, ModelSkeleton};

#[derive(Component)]
pub struct DebugModelSkeletonData {
    polyline_entity: Entity,
    polyline: Handle<Polyline>,
}

#[allow(clippy::type_complexity)]
pub fn debug_model_skeleton_system(
    mut commands: Commands,
    query_update_debug_skeleton: Query<
        (
            Entity,
            &Transform,
            &ModelSkeleton,
            Option<&DebugModelSkeletonData>,
        ),
        With<DebugModelSkeleton>,
    >,
    query_remove_debug_skeleton: Query<
        (Entity, &DebugModelSkeletonData),
        Without<DebugModelSkeleton>,
    >,
    query_bone: Query<(&GlobalTransform, Option<&Parent>)>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
) {
    for (entity, model_transform, model_skeleton, debug_skeleton) in
        query_update_debug_skeleton.iter()
    {
        let transform = Transform::from_matrix(model_transform.compute_matrix().inverse());

        if let Some(debug_skeleton) = debug_skeleton.as_ref() {
            // Update existing debug skeleton
            if let Some(polyline) = polylines.get_mut(debug_skeleton.polyline.clone()) {
                polyline.vertices.clear();

                for bone_entity in model_skeleton.bones.iter() {
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

            commands
                .entity(debug_skeleton.polyline_entity)
                .insert(transform);
        } else {
            // Add a new debug skeleton
            let mut polyline_vertices = Vec::new();
            for bone_entity in model_skeleton.bones.iter() {
                if let Ok((transform, parent)) = query_bone.get(*bone_entity) {
                    if let Some((parent_transform, _)) =
                        parent.and_then(|x| query_bone.get(x.0).ok())
                    {
                        polyline_vertices.push(transform.translation);
                        polyline_vertices.push(parent_transform.translation);
                        polyline_vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
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
                        color: Color::PINK,
                        perspective: false,
                        depth_test: false,
                    }),
                    transform,
                    ..Default::default()
                })
                .id();

            commands
                .entity(entity)
                .insert(DebugModelSkeletonData {
                    polyline_entity,
                    polyline,
                })
                .add_child(polyline_entity);
        }
    }

    for (entity, debug_skeleton) in query_remove_debug_skeleton.iter() {
        commands.entity(debug_skeleton.polyline_entity).despawn();
        commands.entity(entity).remove::<DebugModelSkeletonData>();
    }
}
