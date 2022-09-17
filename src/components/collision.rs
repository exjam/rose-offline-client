use bevy::{
    ecs::system::EntityCommands,
    prelude::{Component, Entity, World},
};
use bevy_inspector_egui::Inspectable;

#[derive(Component, Inspectable)]
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

#[derive(Component, Inspectable)]
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

pub const COLLISION_GROUP_ZONE_OBJECT: u32 = 1 << 0;
pub const COLLISION_GROUP_ZONE_TERRAIN: u32 = 1 << 1;
pub const COLLISION_GROUP_ZONE_WATER: u32 = 1 << 2;
pub const COLLISION_GROUP_ZONE_EVENT_OBJECT: u32 = 1 << 3;
pub const COLLISION_GROUP_ZONE_WARP_OBJECT: u32 = 1 << 4;
pub const COLLISION_GROUP_PHYSICS_TOY: u32 = 1 << 5;

pub const COLLISION_GROUP_PLAYER: u32 = 1 << 9;
pub const COLLISION_GROUP_CHARACTER: u32 = 1 << 10;
pub const COLLISION_GROUP_NPC: u32 = 1 << 11;
pub const COLLISION_GROUP_ITEM_DROP: u32 = 1 << 12;

pub const COLLISION_FILTER_INSPECTABLE: u32 = 1 << 16;
pub const COLLISION_FILTER_COLLIDABLE: u32 = 1 << 17;
pub const COLLISION_FILTER_CLICKABLE: u32 = 1 << 18;
pub const COLLISION_FILTER_MOVEABLE: u32 = 1 << 19;
