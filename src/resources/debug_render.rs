use bevy::{
    math::Vec3,
    prelude::{Entity, Handle},
};
use bevy_polyline::prelude::Polyline;

#[derive(Default)]
pub struct DebugRenderConfig {
    pub colliders: bool,
    pub skeleton: bool,
    pub bone_up: bool,
}

pub struct DebugRenderPolyline {
    pub entity: Entity,
    pub polyline: Handle<Polyline>,
    pub vertices: Vec<Vec3>,
}

pub struct DebugRenderColliderData {
    pub collider: Vec<DebugRenderPolyline>,
}

pub struct DebugRenderSkeletonData {
    pub skeleton: DebugRenderPolyline,
    pub bone_up: DebugRenderPolyline,
}
