use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

use rose_file_readers::ZscCollisionShape;

#[derive(Clone, Reflect, FromReflect)]
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

#[derive(Clone, Default, Reflect, FromReflect)]
pub struct ZoneObjectId {
    pub ifo_object_id: usize,
    pub zsc_object_id: usize,
}

#[derive(Clone, Default, Reflect, FromReflect)]
pub struct ZoneObjectPart {
    pub ifo_object_id: usize,
    pub zsc_object_id: usize,
    pub zsc_part_id: usize,
    pub mesh_path: String,
    pub collision_shape: ZoneObjectPartCollisionShape,
    pub collision_not_moveable: bool,
    pub collision_not_pickable: bool,
    pub collision_height_only: bool,
    pub collision_no_camera: bool,
}

#[derive(Clone, Default, Reflect, FromReflect)]
pub struct ZoneObjectAnimatedObject {
    pub mesh_path: String,
    pub motion_path: String,
    pub texture_path: String,
}

#[derive(Clone, Default, Reflect, FromReflect)]
pub struct ZoneObjectTerrain {
    pub block_x: u32,
    pub block_y: u32,
}

#[derive(Clone, Component, Default, Reflect, FromReflect)]
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
    #[default]
    Water,
}
