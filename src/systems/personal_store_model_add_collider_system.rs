use bevy::{
    ecs::query::QueryEntityError,
    math::{Vec3, Vec3A},
    prelude::{
        BuildChildren, Commands, Entity, GlobalTransform, Handle, Mesh, Query, Transform, With,
        Without,
    },
    render::primitives::Aabb,
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};

use crate::components::{
    ColliderEntity, ColliderParent, ModelHeight, PersonalStoreModel, PlayerCharacter,
    COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_CHARACTER,
    COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_PLAYER,
};

pub fn personal_store_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<
        (Entity, &PersonalStoreModel, Option<&PlayerCharacter>),
        Without<ColliderEntity>,
    >,
    query_aabb: Query<Option<&Aabb>, With<Handle<Mesh>>>,
) {
    for (entity, personal_store_model, player_character) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        for part_entity in personal_store_model.model_parts.iter() {
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

        if min.is_none() || max.is_none() || !all_parts_loaded {
            // Try again next frame
            continue;
        }
        let min = Vec3::from(min.unwrap());
        let max = Vec3::from(max.unwrap());
        let half_extents = 0.5 * (max - min);

        let collider_entity = commands
            .spawn((
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                ColliderParent::new(entity),
                CollisionGroups::new(
                    if player_character.is_some() {
                        COLLISION_GROUP_PLAYER
                    } else {
                        COLLISION_GROUP_CHARACTER
                    },
                    COLLISION_FILTER_INSPECTABLE
                        | COLLISION_FILTER_CLICKABLE
                        | COLLISION_GROUP_PHYSICS_TOY,
                ),
                Transform::from_translation(Vec3::new(0.0, half_extents.y - min.y, 0.0)),
                GlobalTransform::default(),
            ))
            .id();

        commands
            .entity(personal_store_model.model)
            .add_child(collider_entity);

        commands.entity(entity).insert((
            ColliderEntity::new(collider_entity),
            ModelHeight::new(0.65 + half_extents.y * 2.0),
        ));
    }
}
