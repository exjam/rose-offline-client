use bevy::{prelude::Component, reflect::Reflect};

use rose_file_readers::ZscCollisionShape;

#[derive(Clone, Reflect)]
pub enum ZoneObjectPartCollisionShape {
    None,
    Sphere,
    AxisAlignedBoundingBox,
    ObjectOrientedBoundingBox,
    Polygon,
}

impl Default for ZoneObjectPartCollisionShape {
    fn default() -> Self {
        Self::AxisAlignedBoundingBox
    }
}

impl From<&Option<ZscCollisionShape>> for ZoneObjectPartCollisionShape {
    fn from(value: &Option<ZscCollisionShape>) -> Self {
        match value {
            Some(ZscCollisionShape::Sphere) => Self::Sphere,
            Some(ZscCollisionShape::AxisAlignedBoundingBox) => Self::AxisAlignedBoundingBox,
            Some(ZscCollisionShape::ObjectOrientedBoundingBox) => Self::ObjectOrientedBoundingBox,
            Some(ZscCollisionShape::Polygon) => Self::Polygon,
            None => Self::None,
        }
    }
}

#[derive(Clone, Default, Reflect)]
pub struct ZoneObjectId {
    pub id: usize,
}

#[derive(Clone, Default, Reflect)]
pub struct ZoneObjectPart {
    pub mesh_path: String,
    pub collision_shape: ZoneObjectPartCollisionShape,
    pub collision_not_moveable: bool,
    pub collision_not_pickable: bool,
    pub collision_height_only: bool,
    pub collision_no_camera: bool,
}

#[derive(Clone, Default, Reflect)]
pub struct ZoneObjectAnimatedObject {
    pub mesh_path: String,
    pub motion_path: String,
    pub texture_path: String,
}

#[derive(Clone, Default, Reflect)]
pub struct ZoneObjectTerrain {
    pub block_x: u32,
    pub block_y: u32,
}

#[derive(Clone, Component, Reflect)]
pub enum ZoneObject {
    AnimatedObject(ZoneObjectAnimatedObject),
    WarpObject(ZoneObjectId),
    WarpObjectPart(ZoneObjectPart),
    EventObject(ZoneObjectId),
    EventObjectPart(ZoneObjectPart),
    CnstObject(ZoneObjectId),
    CnstObjectPart(ZoneObjectPart),
    DecoObject(ZoneObjectId),
    DecoObjectPart(ZoneObjectPart),
    Terrain(ZoneObjectTerrain),
    Water,
}
