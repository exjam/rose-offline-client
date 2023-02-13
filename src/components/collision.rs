use bevy::{
    ecs::system::EntityCommands,
    prelude::{Component, Entity, World},
    reflect::Reflect,
};
use bevy_rapier3d::prelude::Group;

#[derive(Component, Reflect)]
pub struct ColliderEntity {
    pub entity: Entity,
}

impl ColliderEntity {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

pub trait RemoveColliderCommand {
    fn remove_and_despawn_collider(&mut self) -> &mut Self;
}

impl<'w, 's, 'a> RemoveColliderCommand for EntityCommands<'w, 's, 'a> {
    fn remove_and_despawn_collider(&mut self) -> &mut Self {
        let entity = self.id();

        self.commands().add(move |world: &mut World| {
            let mut world_entity = world.entity_mut(entity);
            if let Some(collider_entity) = world_entity.get::<ColliderEntity>() {
                let collider_entity = collider_entity.entity;
                world_entity.remove::<ColliderEntity>();
                world.despawn(collider_entity);
            }
        });

        self
    }
}

#[derive(Component, Reflect)]
pub struct ColliderParent {
    pub entity: Entity,
}

impl ColliderParent {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

#[derive(Component)]
pub struct CollisionPlayer;

#[derive(Component)]
pub struct CollisionHeightOnly;

pub const COLLISION_GROUP_ZONE_OBJECT: Group = Group::from_bits_truncate(1 << 0);
pub const COLLISION_GROUP_ZONE_TERRAIN: Group = Group::from_bits_truncate(1 << 1);
pub const COLLISION_GROUP_ZONE_WATER: Group = Group::from_bits_truncate(1 << 2);
pub const COLLISION_GROUP_ZONE_EVENT_OBJECT: Group = Group::from_bits_truncate(1 << 3);
pub const COLLISION_GROUP_ZONE_WARP_OBJECT: Group = Group::from_bits_truncate(1 << 4);
pub const COLLISION_GROUP_PHYSICS_TOY: Group = Group::from_bits_truncate(1 << 5);

pub const COLLISION_GROUP_PLAYER: Group = Group::from_bits_truncate(1 << 9);
pub const COLLISION_GROUP_CHARACTER: Group = Group::from_bits_truncate(1 << 10);
pub const COLLISION_GROUP_NPC: Group = Group::from_bits_truncate(1 << 11);
pub const COLLISION_GROUP_ITEM_DROP: Group = Group::from_bits_truncate(1 << 12);

pub const COLLISION_FILTER_INSPECTABLE: Group = Group::from_bits_truncate(1 << 16);
pub const COLLISION_FILTER_COLLIDABLE: Group = Group::from_bits_truncate(1 << 17);
pub const COLLISION_FILTER_CLICKABLE: Group = Group::from_bits_truncate(1 << 18);
pub const COLLISION_FILTER_MOVEABLE: Group = Group::from_bits_truncate(1 << 19);
