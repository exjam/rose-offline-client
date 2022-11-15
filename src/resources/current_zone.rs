use bevy::prelude::{Handle, Resource};

use rose_data::ZoneId;

use crate::zone_loader::ZoneLoaderAsset;

#[derive(Resource)]
pub struct CurrentZone {
    pub id: ZoneId,
    pub handle: Handle<ZoneLoaderAsset>,
}
