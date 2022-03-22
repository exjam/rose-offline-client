use bevy::prelude::{Component, Handle};

use crate::zmo_asset_loader::ZmoAsset;

#[derive(Component, Default)]
pub struct ActiveMotion {
    pub start_time: f64,
    pub motion: Handle<ZmoAsset>,
    pub repeat_limit: Option<usize>, // If None, repeats forever
    pub complete: bool,
}

impl ActiveMotion {
    pub fn new(motion: Handle<ZmoAsset>, start_time: f64) -> Self {
        Self {
            motion,
            start_time,
            repeat_limit: None,
            complete: false,
        }
    }

    pub fn with_repeat_limit(mut self, repeat_limit: usize) -> Self {
        self.repeat_limit = Some(repeat_limit);
        self
    }
}
