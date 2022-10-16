use bevy::prelude::{Component, Entity};
use enum_map::EnumMap;
use rose_data::VehiclePartIndex;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VehicleSoundState {
    Idle,
    Move,
}

#[derive(Component)]
pub struct VehicleSound {
    pub state: VehicleSoundState,
    pub sound_entity: EnumMap<VehiclePartIndex, Entity>,
}
