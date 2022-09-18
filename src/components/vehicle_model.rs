use bevy::prelude::{Component, Entity, Handle};
use enum_map::EnumMap;

use rose_data::{VehicleMotionAction, VehiclePartIndex};

use crate::zmo_asset_loader::ZmoAsset;

#[derive(Component)]
pub struct VehicleModel {
    pub model_parts: EnumMap<VehiclePartIndex, (usize, Vec<Entity>)>,
    pub vehicle_action_motions: EnumMap<VehicleMotionAction, Handle<ZmoAsset>>,
    pub character_action_motions: EnumMap<VehicleMotionAction, Handle<ZmoAsset>>,
}
