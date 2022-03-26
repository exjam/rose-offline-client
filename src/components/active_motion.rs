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
    pub fn new_repeating(motion: Handle<ZmoAsset>, start_time: f64) -> Self {
        Self {
            motion,
            start_time,
            repeat_limit: None,
            complete: false,
        }
    }

    pub fn new_once(motion: Handle<ZmoAsset>, start_time: f64) -> Self {
        Self {
            motion,
            start_time,
            repeat_limit: Some(1),
            complete: false,
        }
    }

    #[allow(dead_code)]
    pub fn new_repeat_n(motion: Handle<ZmoAsset>, start_time: f64, repeat_count: usize) -> Self {
        Self {
            motion,
            start_time,
            repeat_limit: Some(repeat_count),
            complete: false,
        }
    }
}
