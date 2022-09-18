use bevy::{
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, ComputedVisibility,
        DespawnRecursiveExt, Entity, GlobalTransform, Query, Res, ResMut, Transform, Visibility,
    },
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};

use rose_game_common::components::{Equipment, MoveMode};

use crate::{
    components::{DummyBoneOffset, Vehicle, VehicleModel},
    model_loader::ModelLoader,
    render::{EffectMeshMaterial, ObjectMaterial, ParticleMaterial},
};

pub fn vehicle_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Equipment,
            &MoveMode,
            &SkinnedMesh,
            Option<&Vehicle>,
        ),
        Changed<MoveMode>,
    >,
    query_vehicle: Query<(&VehicleModel, &SkinnedMesh, &DummyBoneOffset)>,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Vehicle entity, where entity becomes a child of it.
    for (entity, equipment, move_mode, skinned_mesh, vehicle) in query.iter_mut() {
        if let Some(vehicle) = vehicle {
            if matches!(move_mode, MoveMode::Drive) {
                // TODO: Update vehicle model if equipment changed
                continue;
            }

            // Reparent character model to entity root
            let (_, vehicle_skinned_mesh, vehicle_dummy_bone_offset) =
                query_vehicle.get(vehicle.entity).unwrap();
            let vehicle_character_model_dummy =
                vehicle_skinned_mesh.joints[vehicle_dummy_bone_offset.index];
            commands
                .entity(vehicle_character_model_dummy)
                .remove_children(&[skinned_mesh.joints[0]]);
            commands.entity(entity).add_child(skinned_mesh.joints[0]);

            // Despawn vehicle
            commands.entity(vehicle.entity).despawn_recursive();
            commands.entity(entity).remove::<Vehicle>();
        } else if matches!(move_mode, MoveMode::Drive) {
            let vehicle_entity = commands
                .spawn_bundle((
                    Visibility::visible(),
                    ComputedVisibility::default(),
                    Transform::default(),
                    GlobalTransform::default(),
                ))
                .id();

            // Spawn new cart model
            let (vehicle_model, vehicle_skinned_mesh, vehicle_dummy_bone_offset) = model_loader
                .spawn_vehicle_model(
                    &mut commands,
                    &asset_server,
                    &mut object_materials,
                    &mut particle_materials,
                    &mut effect_mesh_materials,
                    &mut skinned_mesh_inverse_bindposes_assets,
                    vehicle_entity,
                    equipment,
                );

            commands
                .entity(entity)
                .add_child(vehicle_entity)
                .insert(Vehicle {
                    entity: vehicle_entity,
                    action_motions: vehicle_model.character_action_motions.clone(),
                });
            let vehicle_character_model_dummy =
                vehicle_skinned_mesh.joints[vehicle_dummy_bone_offset.index];
            commands.entity(vehicle_entity).insert_bundle((
                vehicle_model,
                vehicle_skinned_mesh,
                vehicle_dummy_bone_offset,
            ));

            // Reparent character model to vehicle dummy bone
            commands
                .entity(entity)
                .remove_children(&[skinned_mesh.joints[0]]);
            commands
                .entity(vehicle_character_model_dummy)
                .add_child(skinned_mesh.joints[0]);
        }
    }
}
