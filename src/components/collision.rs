use bevy::prelude::Component;

#[derive(Component)]
pub struct CollisionTriMesh {
    pub group: u32,
    pub filter: u32,
}

#[derive(Component)]
pub struct CollisionRayCastSource;

pub const COLLISION_GROUP_ZONE_OBJECT: u32 = 1 << 0;
pub const COLLISION_GROUP_ZONE_TERRAIN: u32 = 1 << 1;
pub const COLLISION_GROUP_ZONE_WATER: u32 = 1 << 2;
#[allow(dead_code)]
pub const COLLISION_GROUP_CHARACTER: u32 = 1 << 3;
#[allow(dead_code)]
pub const COLLISION_GROUP_NPC: u32 = 1 << 4;

pub const COLLISION_FILTER_INSPECTABLE: u32 = 1 << 16;
pub const COLLISION_FILTER_COLLIDABLE: u32 = 1 << 17;
pub const COLLISION_FILTER_CLICKABLE: u32 = 1 << 18;
