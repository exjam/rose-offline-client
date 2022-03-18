use rose_data::ZoneId;

pub struct LoadZoneEvent {
    pub id: ZoneId,
}

impl LoadZoneEvent {
    pub fn new(id: ZoneId) -> Self {
        Self { id }
    }
}

pub enum ZoneEvent {
    Loaded(ZoneId),
}
