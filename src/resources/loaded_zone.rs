use bevy::prelude::Entity;
use rose_data::ZoneId;

#[derive(Default)]
pub struct LoadedZone {
    pub zone: Option<(ZoneId, Entity)>,
    pub next_zone_id: Option<ZoneId>,
}

impl LoadedZone {
    pub fn with_next_zone(zone_id: ZoneId) -> Self {
        Self {
            zone: None,
            next_zone_id: Some(zone_id),
        }
    }
}
