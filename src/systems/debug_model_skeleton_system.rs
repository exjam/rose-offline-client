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
use rand::prelude::SliceRandom;

use crate::components::DebugModelSkeleton;

#[derive(Component)]
pub struct DebugModelSkeletonData {
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

#[allow(clippy::type_complexity)]
pub fn debug_model_skeleton_system(
    mut commands: Commands,
    query_update_debug_skeleton: Query<
        (
            Entity,
            &Transform,
            &SkinnedMesh,
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

            commands
                .entity(debug_skeleton.polyline_entity)
                .insert(transform);
        } else {
            // Add a new debug skeleton
            let mut polyline_vertices = Vec::new();
            for bone_entity in skinned_mesh.joints.iter() {
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
