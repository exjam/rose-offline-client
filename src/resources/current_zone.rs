use bevy::prelude::Handle;

use rose_data::ZoneId;

use crate::zone_loader::ZoneLoaderAsset;

pub struct CurrentZone {
    pub id: ZoneId,
    pub handle: Handle<ZoneLoaderAsset>,
}
