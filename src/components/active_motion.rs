use bevy::prelude::{Component, Handle};

use crate::zmo_asset_loader::ZmoAsset;

#[derive(Component, Default)]
pub struct ActiveMotion {
    pub start_time: f64,
    pub motion: Handle<ZmoAsset>,
}

impl ActiveMotion {
    pub fn new(motion: Handle<ZmoAsset>, start_time: f64) -> Self {
        Self { motion, start_time }
    }
}
