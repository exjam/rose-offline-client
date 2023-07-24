use bevy::prelude::Event;

use rose_data::ZoneId;

#[derive(Event)]
pub struct LoadZoneEvent {
    pub id: ZoneId,
    pub despawn_other_zones: bool,
}

impl LoadZoneEvent {
    pub fn new(id: ZoneId) -> Self {
        Self {
            id,
            despawn_other_zones: true,
        }
    }
}

#[derive(Event)]
pub enum ZoneEvent {
    Loaded(ZoneId),
}
