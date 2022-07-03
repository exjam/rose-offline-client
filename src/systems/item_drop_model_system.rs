use bevy::{
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, Component, Entity, GlobalTransform,
        Handle, Mesh, Query, Res, ResMut, Transform, With, Without,
    },
    render::primitives::Aabb,
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};

use rose_game_common::components::ItemDrop;

use crate::{
    components::{
        ActiveMotion, ColliderEntity, ColliderParent, ItemDropModel, COLLISION_FILTER_CLICKABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_ITEM_DROP,
    },
    model_loader::ModelLoader,
    render::ObjectMaterial,
};

pub fn item_drop_model_system(
    mut commands: Commands,
    mut query: Query<(Entity, &ItemDrop, Option<&mut ItemDropModel>), Changed<ItemDrop>>,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
) {
    for (entity, item_drop, mut current_item_drop_model) in query.iter_mut() {
        if let Some(current_item_drop_model) = current_item_drop_model.as_mut() {
            if current_item_drop_model.dropped_item == item_drop.item {
                // Does not need new model, ignore
                continue;
            }

            // Despawn model parts
            for part_entity in current_item_drop_model.model_parts.iter() {
                commands.entity(*part_entity).despawn();
            }
        }

        let (item_drop_model, drop_motion) = model_loader.spawn_item_drop_model(
            &mut commands,
            &asset_server,
            &mut object_materials,
            entity,
            item_drop.item.as_ref(),
        );

        let root_model_bone = item_drop_model.root_bone;
        commands
            .entity(root_model_bone)
            .insert_bundle((ActiveMotion::new_once(drop_motion),));

        commands.entity(entity).insert(item_drop_model);
    }
}

#[derive(Component)]
pub struct ItemDropColliderOffset {
    pub offset: Vec3,
}

pub fn item_drop_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &ItemDropModel, &GlobalTransform), Without<ColliderEntity>>,
    query_collider: Query<(&GlobalTransform, &ItemDropColliderOffset, &ColliderEntity)>,
    mut query_transform: Query<&mut Transform>,
    query_aabb: Query<Option<&Aabb>, With<Handle<Mesh>>>,
) {
    // Add colliders to NPC models without one
    for (entity, item_drop_model, global_transform) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        // Collect the AABB of mesh parts
        for part_entity in item_drop_model.model_parts.iter() {
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

        if min.is_none() || max.is_none() || !all_parts_loaded {
            continue;
        }
        let min = min.unwrap();
        let max = max.unwrap();
        let local_bound_center = 0.5 * (min + max);
        let half_extents = Vec3::from(0.5 * (max - min));
        let collider_offset = Vec3::from(local_bound_center);

        let collider_entity = commands
            .spawn_bundle((
                ColliderParent::new(entity),
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                CollisionGroups::new(
                    COLLISION_GROUP_ITEM_DROP,
                    COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_CLICKABLE,
                ),
                Transform::from_translation(global_transform.translation + collider_offset)
                    .with_rotation(
                        global_transform.rotation
                            * Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0),
                    ),
                GlobalTransform::default(),
            ))
            .id();

        commands
            .entity(entity)
            .insert_bundle((
                ColliderEntity::new(collider_entity),
                ItemDropColliderOffset {
                    offset: collider_offset,
                },
            ))
            .add_child(collider_entity);
    }

    // Update any existing collider's position
    for (global_transform, collider_offset, collider_entity) in query_collider.iter() {
        if let Ok(mut collider_transform) = query_transform.get_mut(collider_entity.entity) {
            collider_transform.translation = global_transform.translation + collider_offset.offset;
            collider_transform.rotation = global_transform.rotation
                * Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0);
        }
    }
}
