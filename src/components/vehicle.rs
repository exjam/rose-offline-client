use bevy::prelude::{Component, Entity, Handle};
use enum_map::EnumMap;

use rose_data::VehicleMotionAction;

use crate::zmo_asset_loader::ZmoAsset;

#[derive(Component)]
pub struct Vehicle {
    pub entity: Entity,
    pub action_motions: EnumMap<VehicleMotionAction, Handle<ZmoAsset>>,
}
