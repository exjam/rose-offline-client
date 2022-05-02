use bevy::{
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, Component, Entity, GlobalTransform,
        Query, Res, ResMut, Transform, With, Without,
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
        ColliderEntity, NpcModel, COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_INSPECTABLE,
        COLLISION_GROUP_NPC,
    },
    model_loader::ModelLoader,
    render::{EffectMeshMaterial, ParticleMaterial, StaticMeshMaterial},
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
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
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
            &mut static_mesh_materials,
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

#[derive(Component)]
pub struct NpcColliderRootBoneOffset {
    pub offset: Vec3,
}

pub fn npc_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &NpcModel, &SkinnedMesh), Without<ColliderEntity>>,
    query_collider: Query<(&SkinnedMesh, &NpcColliderRootBoneOffset, &ColliderEntity)>,
    query_aabb: Query<Option<&Aabb>, With<SkinnedMesh>>,
    query_global_transform: Query<&GlobalTransform>,
    mut query_transform: Query<&mut Transform>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Add colliders to NPC models without one
    for (entity, npc_model, skinned_mesh) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        // Collect the AABB of skinned mesh parts
        for part_entity in npc_model.model_parts.iter() {
            if let Ok(aabb) = query_aabb.get(*part_entity) {
                if let Some(aabb) = aabb {
                    min = Some(min.map_or_else(|| aabb.min(), |min| min.min(aabb.min())));
                    max = Some(max.map_or_else(|| aabb.max(), |max| max.max(aabb.max())));
                } else {
                    all_parts_loaded = false;
                    break;
                }
            }
        }

        let inverse_bindpose = inverse_bindposes.get(&skinned_mesh.inverse_bindposes);
        let root_bone_global_transform = query_global_transform.get(skinned_mesh.joints[0]).ok();
        if min.is_none()
            || max.is_none()
            || !all_parts_loaded
            || inverse_bindpose.is_none()
            || root_bone_global_transform.is_none()
        {
            continue;
        }
        let root_bone_global_transform = root_bone_global_transform.unwrap();
        let inverse_bindpose = inverse_bindpose.unwrap();
        let root_bone_local_transform = Transform::from_matrix(inverse_bindpose[0].inverse());

        let min = Vec3::from(min.unwrap()) * root_bone_global_transform.scale;
        let max = Vec3::from(max.unwrap()) * root_bone_global_transform.scale;
        let local_bound_center = 0.5 * (min + max);
        let half_extents = 0.5 * (max - min);
        let root_bone_offset = local_bound_center
            - root_bone_local_transform.translation * root_bone_global_transform.scale;

        let collider_entity = commands
            .spawn_bundle((
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                CollisionGroups::new(
                    COLLISION_GROUP_NPC,
                    COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_CLICKABLE,
                ),
                Transform::from_translation(
                    root_bone_global_transform.translation + root_bone_offset,
                )
                .with_rotation(
                    root_bone_global_transform.rotation
                        * Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0),
                ),
                GlobalTransform::default(),
            ))
            .id();

        commands
            .entity(entity)
            .insert_bundle((
                NpcColliderRootBoneOffset {
                    offset: root_bone_offset,
                },
                ColliderEntity::new(collider_entity),
            ))
            .add_child(collider_entity);
    }

    // Update any existing collider's position
    for (skinned_mesh, root_bone_offset, collider_entity) in query_collider.iter() {
        if let Ok(root_bone_global_transform) = query_global_transform.get(skinned_mesh.joints[0]) {
            if let Ok(mut collider_transform) = query_transform.get_mut(collider_entity.entity) {
                collider_transform.translation =
                    root_bone_global_transform.translation + root_bone_offset.offset;
                collider_transform.rotation = root_bone_global_transform.rotation
                    * Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0);
            }
        }
    }
}
