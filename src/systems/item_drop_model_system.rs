use bevy::{
    ecs::query::QueryEntityError,
    math::{Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, Entity, GlobalTransform, Handle,
        Mesh, Query, Res, ResMut, Transform, With, Without,
    },
    render::primitives::Aabb,
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};

use rose_game_common::components::ItemDrop;

use crate::{
    components::{
        ActiveMotion, ColliderEntity, ColliderParent, ItemDropModel, COLLISION_FILTER_CLICKABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_ITEM_DROP, COLLISION_GROUP_PHYSICS_TOY,
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
            .insert(ActiveMotion::new_once(drop_motion));

        commands.entity(entity).insert(item_drop_model);
    }
}

pub fn item_drop_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &ItemDropModel), Without<ColliderEntity>>,
    query_aabb: Query<Option<&Aabb>, With<Handle<Mesh>>>,
) {
    // Add colliders to NPC models without one
    for (entity, item_drop_model) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        // Collect the AABB of mesh parts
        for part_entity in item_drop_model.model_parts.iter() {
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

        if !all_parts_loaded || min.is_none() || max.is_none() {
            continue;
        }
        let min = Vec3::from(min.unwrap());
        let max = Vec3::from(max.unwrap());

        let local_bound_center = 0.5 * (min + max);
        let half_extents = 0.5 * (max - min);

        let collider_entity = commands
            .spawn((
                ColliderParent::new(entity),
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                CollisionGroups::new(
                    bevy_rapier3d::geometry::Group::from_bits_truncate(COLLISION_GROUP_ITEM_DROP),
                    bevy_rapier3d::geometry::Group::from_bits_truncate(
                        COLLISION_FILTER_INSPECTABLE
                            | COLLISION_FILTER_CLICKABLE
                            | COLLISION_GROUP_PHYSICS_TOY,
                    ),
                ),
                Transform::from_translation(local_bound_center),
                GlobalTransform::default(),
            ))
            .id();

        commands
            .entity(entity)
            .insert(ColliderEntity::new(collider_entity));

        commands
            .entity(item_drop_model.root_bone)
            .add_child(collider_entity);
    }
}
