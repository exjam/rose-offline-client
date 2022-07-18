use bevy::prelude::Component;

use rose_data::ZoneId;

#[derive(Component)]
pub struct Zone {
    pub id: ZoneId,
}
