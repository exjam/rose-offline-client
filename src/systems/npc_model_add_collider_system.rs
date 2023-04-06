use bevy::{
    asset::LoadState,
    ecs::query::QueryEntityError,
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, Entity, GlobalTransform, Query, Res,
        Transform, With, Without,
    },
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        primitives::Aabb,
    },
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};

use rose_data::NpcMotionAction;

use crate::{
    animation::ZmoAsset,
    components::{
        ColliderEntity, ColliderParent, ModelHeight, NpcModel, COLLISION_FILTER_CLICKABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_NPC, COLLISION_GROUP_PHYSICS_TOY,
    },
};

pub fn npc_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &NpcModel, &SkinnedMesh), Without<ColliderEntity>>,
    query_aabb: Query<Option<&Aabb>, With<SkinnedMesh>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    zmo_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
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

        let root_bone_height = if let Some(motion_data) =
            zmo_assets.get(&npc_model.action_motions[NpcMotionAction::Stop])
        {
            // For flying NPCs their t-pose is often on the floor, so we should adjust model
            // height by the difference in the root bone position between the t-pose and the
            // first frame of their idle animation. And for safety, do not allow below 0.0
            (motion_data
                .get_translation(0, 0)
                .map_or(0.0, |translation| translation.y)
                - npc_model.root_bone_position.y)
                .max(0.0)
        } else {
            match asset_server.get_load_state(&npc_model.action_motions[NpcMotionAction::Stop]) {
                LoadState::NotLoaded | LoadState::Loading => all_parts_loaded = false,
                LoadState::Loaded | LoadState::Failed | LoadState::Unloaded => {}
            }
            0.0
        };

        let inverse_bindpose = inverse_bindposes.get(&skinned_mesh.inverse_bindposes);
        if min.is_none()
            || max.is_none()
            || !all_parts_loaded
            || inverse_bindpose.is_none()
            || skinned_mesh.joints.is_empty()
        {
            continue;
        }
        let root_bone_entity = skinned_mesh.joints[0];
        let root_bone_inverse_bindpose = Transform::from_matrix(inverse_bindpose.unwrap()[0]);

        let min = Vec3::from(min.unwrap());
        let max = Vec3::from(max.unwrap());
        let local_bound_center = 0.5 * (min + max);
        let half_extents = 0.5 * (max - min);
        let root_bone_offset = root_bone_inverse_bindpose.transform_point(local_bound_center);

        let collider_entity = commands
            .spawn((
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

        commands.entity(entity).insert((
            ColliderEntity::new(collider_entity),
            ModelHeight::new(root_bone_height + half_extents.y * 2.0),
        ));
    }
}
