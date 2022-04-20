use rose_data::ZoneId;

pub struct CurrentZone {
    pub id: ZoneId,
}

impl CurrentZone {
    pub fn new(id: ZoneId) -> Self {
        Self { id }
    }
}
