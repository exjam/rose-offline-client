use bevy::{
    math::Vec3,
    prelude::{Entity, Handle, Resource},
};
use bevy_polyline::prelude::Polyline;

#[derive(Default, Resource)]
pub struct DebugRenderConfig {
    pub colliders: bool,
    pub skeleton: bool,
    pub bone_up: bool,
    pub directional_light_frustum: bool,
}

#[derive(Resource)]
pub struct DebugRenderPolyline {
    pub entity: Entity,
    pub polyline: Handle<Polyline>,
    pub vertices: Vec<Vec3>,
}

#[derive(Resource)]
pub struct DebugRenderColliderData {
    pub collider: Vec<DebugRenderPolyline>,
}

#[derive(Resource)]
pub struct DebugRenderSkeletonData {
    pub skeleton: DebugRenderPolyline,
    pub bone_up: DebugRenderPolyline,
}

#[derive(Resource)]
pub struct DebugRenderDirectionalLightData {
    pub frustum: DebugRenderPolyline,
}
