use bevy::{
    ecs::query::QueryEntityError,
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, Entity, GlobalTransform, Query, Res,
        ResMut, Transform, With, Without,
    },
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        primitives::Aabb,
    },
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};
use enum_map::EnumMap;

use rose_game_common::components::Npc;

use crate::{
    components::{
        ColliderEntity, ColliderParent, NpcModel, COLLISION_FILTER_CLICKABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_NPC, COLLISION_GROUP_PHYSICS_TOY,
    },
    model_loader::ModelLoader,
    render::{EffectMeshMaterial, ObjectMaterial, ParticleMaterial},
    resources::GameData,
};

pub fn npc_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Npc,
            Option<&mut NpcModel>,
            Option<&SkinnedMesh>,
            &Transform,
            Option<&ColliderEntity>,
        ),
        Changed<Npc>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    game_data: Res<GameData>,
) {
    for (entity, npc, mut current_npc_model, skinned_mesh, transform, collider_entity) in
        query.iter_mut()
    {
        if let Some(current_npc_model) = current_npc_model.as_mut() {
            if current_npc_model.npc_id == npc.id {
                // Does not need new model, ignore
                continue;
            }

            // Despawn model parts
            for part_entity in current_npc_model.model_parts.iter() {
                commands.entity(*part_entity).despawn();
            }

            // Despawn model skeleton
            if let Some(skinned_mesh) = skinned_mesh {
                for bone_entity in skinned_mesh.joints.iter() {
                    commands.entity(*bone_entity).despawn();
                }
            }

            if let Some(collider_entity) = collider_entity {
                commands.entity(entity).remove::<ColliderEntity>();
                commands.entity(collider_entity.entity).despawn();
            }
        }

        if let Some((npc_model, skinned_mesh, dummy_bone_offset)) = model_loader.spawn_npc_model(
            &mut commands,
            &asset_server,
            &mut effect_mesh_materials,
            &mut particle_materials,
            &mut object_materials,
            &mut skinned_mesh_inverse_bindposes_assets,
            entity,
            npc.id,
        ) {
            let transform = if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                transform.with_scale(Vec3::new(npc_data.scale, npc_data.scale, npc_data.scale))
            } else {
                *transform
            };

            commands.entity(entity).insert_bundle((
                npc_model,
                skinned_mesh,
                dummy_bone_offset,
                transform,
            ));
        } else {
            commands
                .entity(entity)
                .insert(NpcModel {
                    npc_id: npc.id,
                    model_parts: Vec::new(),
                    action_motions: EnumMap::default(),
                })
                .remove::<SkinnedMesh>();
        }
    }
}

pub fn npc_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &NpcModel, &SkinnedMesh), Without<ColliderEntity>>,
    query_aabb: Query<Option<&Aabb>, With<SkinnedMesh>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Add colliders to NPC models without one
    for (entity, npc_model, skinned_mesh) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        // Collect the AABB of skinned mesh parts
        for part_entity in npc_model.model_parts.iter() {
            match query_aabb.get(*part_entity) {
                Ok(Some(aabb)) => {
                    min = Some(min.map_or_else(|| aabb.min(), |min| min.min(aabb.min())));
                    max = Some(max.map_or_else(|| aabb.max(), |max| max.max(aabb.max())));
                }
                Ok(None) | Err(QueryEntityError::NoSuchEntity(_)) => {
                    all_parts_loaded = false;
                    break;
                }
                _ => {}
            }
        }

        let inverse_bindpose = inverse_bindposes.get(&skinned_mesh.inverse_bindposes);
        let root_bone_entity = skinned_mesh.joints[0];
        if min.is_none() || max.is_none() || !all_parts_loaded || inverse_bindpose.is_none() {
            continue;
        }
        let root_bone_inverse_bindpose = Transform::from_matrix(inverse_bindpose.unwrap()[0]);

        let min = Vec3::from(min.unwrap());
        let max = Vec3::from(max.unwrap());
        let local_bound_center = 0.5 * (min + max);
        let half_extents = 0.5 * (max - min);
        let root_bone_offset = root_bone_inverse_bindpose.mul_vec3(local_bound_center);

        let collider_entity = commands
            .spawn_bundle((
                ColliderParent::new(entity),
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                CollisionGroups::new(
                    COLLISION_GROUP_NPC,
                    COLLISION_FILTER_INSPECTABLE
                        | COLLISION_FILTER_CLICKABLE
                        | COLLISION_GROUP_PHYSICS_TOY,
                ),
                Transform::from_translation(root_bone_offset)
                    .with_rotation(Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0)),
                GlobalTransform::default(),
            ))
            .id();

        commands.entity(root_bone_entity).add_child(collider_entity);

        commands
            .entity(entity)
            .insert_bundle((ColliderEntity::new(collider_entity),));
    }
}
