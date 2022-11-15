use bevy::{
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, ComputedVisibility,
        DespawnRecursiveExt, Entity, GlobalTransform, Query, Res, ResMut, Transform, Visibility,
        World,
    },
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};

use rose_game_common::components::{Equipment, MoveMode};

use crate::{
    components::{ActiveMotion, DummyBoneOffset, Vehicle, VehicleModel},
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
    query_vehicle_model: Query<&VehicleModel>,
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
            let vehicle_model = query_vehicle_model
                .get(vehicle.vehicle_model_entity)
                .unwrap();

            if matches!(move_mode, MoveMode::Drive) {
                // TODO: Update vehicle model if equipment changed
                continue;
            }

            // Reparent driver character model to entity root
            commands
                .entity(vehicle_model.driver_dummy_entity)
                .remove_children(&[vehicle_model.driver_model_entity]);
            commands
                .entity(entity)
                .add_child(vehicle_model.driver_model_entity);

            // Move driver ActiveMotion, SkinnedMesh, DummyBoneOffset to root entity
            let driver_model_entity = vehicle_model.driver_model_entity;
            commands.add(move |world: &mut World| {
                let mut driver_model_entity_mut = world.entity_mut(driver_model_entity);
                let character_active_motion = driver_model_entity_mut.remove::<ActiveMotion>();
                let character_dummy_bone_offset =
                    driver_model_entity_mut.remove::<DummyBoneOffset>();
                let character_skinned_mesh = driver_model_entity_mut.remove::<SkinnedMesh>();

                let mut root_entity_mut = world.entity_mut(entity);
                if let Some(character_active_motion) = character_active_motion {
                    root_entity_mut.insert(character_active_motion);
                }
                if let Some(character_dummy_bone_offset) = character_dummy_bone_offset {
                    root_entity_mut.insert(character_dummy_bone_offset);
                }
                if let Some(character_skinned_mesh) = character_skinned_mesh {
                    root_entity_mut.insert(character_skinned_mesh);
                }
            });

            // Despawn vehicle model
            commands
                .entity(vehicle.vehicle_model_entity)
                .despawn_recursive();
            commands.entity(entity).remove::<Vehicle>();
        } else if matches!(move_mode, MoveMode::Drive) {
            let driver_model_entity = skinned_mesh.joints[0];
            let vehicle_model_entity = commands
                .spawn((
                    Visibility::VISIBLE,
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
                    vehicle_model_entity,
                    driver_model_entity,
                    equipment,
                );

            commands
                .entity(entity)
                .add_child(vehicle_model_entity)
                .insert(Vehicle {
                    driver_model_entity,
                    vehicle_model_entity,
                    action_motions: vehicle_model.character_action_motions.clone(),
                });

            // Reparent character model to vehicle dummy bone
            commands
                .entity(entity)
                .remove_children(&[vehicle_model.driver_model_entity]);
            commands
                .entity(vehicle_model.driver_dummy_entity)
                .add_child(vehicle_model.driver_model_entity);

            commands.add(move |world: &mut World| {
                // Move character ActiveMotion, DummyBoneOffset, SkinnedMesh to character model
                let mut root_entity_mut = world.entity_mut(entity);
                let character_active_motion = root_entity_mut.remove::<ActiveMotion>();
                let character_dummy_bone_offset = root_entity_mut.remove::<DummyBoneOffset>();
                let character_skinned_mesh = root_entity_mut.remove::<SkinnedMesh>();

                let mut driver_model_entity_mut =
                    world.entity_mut(vehicle_model.driver_model_entity);
                if let Some(character_active_motion) = character_active_motion {
                    driver_model_entity_mut.insert(character_active_motion);
                }
                if let Some(character_dummy_bone_offset) = character_dummy_bone_offset {
                    driver_model_entity_mut.insert(character_dummy_bone_offset);
                }
                if let Some(character_skinned_mesh) = character_skinned_mesh {
                    driver_model_entity_mut.insert(character_skinned_mesh);
                }

                let mut vehicle_model_entity_mut = world.entity_mut(vehicle_model_entity);
                vehicle_model_entity_mut.insert(vehicle_model);

                let mut root_entity_mut = world.entity_mut(entity);
                root_entity_mut.insert((vehicle_skinned_mesh, vehicle_dummy_bone_offset));
            });
        }
    }
}
