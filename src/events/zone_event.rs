use rose_data::ZoneId;

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

pub enum ZoneEvent {
    Loaded(ZoneId),
}
