use bevy::prelude::{Component, Entity, Handle};
use enum_map::EnumMap;

use rose_data::VehicleMotionAction;

use crate::animation::ZmoAsset;

#[derive(Component)]
pub struct Vehicle {
    pub driver_model_entity: Entity,
    pub vehicle_model_entity: Entity,
    pub action_motions: EnumMap<VehicleMotionAction, Handle<ZmoAsset>>,
}
