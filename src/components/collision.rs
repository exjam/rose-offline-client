use bevy::prelude::{Component, Entity};

#[derive(Component)]
pub struct CollisionTriMesh {
    pub group: u32,
    pub filter: u32,
}

#[derive(Component)]
pub struct MovementCollisionEntities {
    pub down_ray_cast_source: Option<Entity>,
    pub forward_ray_cast_source: Option<Entity>,
}

impl MovementCollisionEntities {
    pub fn new(
        down_ray_cast_source: Option<Entity>,
        forward_ray_cast_source: Option<Entity>,
    ) -> Self {
        Self {
            down_ray_cast_source,
            forward_ray_cast_source,
        }
    }
}

#[derive(Component)]
pub struct CollisionRayCastSource;

pub const COLLISION_GROUP_ZONE_OBJECT: u32 = 1 << 0;
pub const COLLISION_GROUP_ZONE_TERRAIN: u32 = 1 << 1;
pub const COLLISION_GROUP_ZONE_WATER: u32 = 1 << 2;
pub const COLLISION_GROUP_ZONE_EVENT_OBJECT: u32 = 1 << 3;
pub const COLLISION_GROUP_ZONE_WARP_OBJECT: u32 = 1 << 4;

pub const COLLISION_GROUP_CHARACTER: u32 = 1 << 10;
pub const COLLISION_GROUP_NPC: u32 = 1 << 11;
pub const COLLISION_GROUP_ITEM_DROP: u32 = 1 << 12;

pub const COLLISION_FILTER_INSPECTABLE: u32 = 1 << 16;
pub const COLLISION_FILTER_COLLIDABLE: u32 = 1 << 17;
pub const COLLISION_FILTER_CLICKABLE: u32 = 1 << 18;
