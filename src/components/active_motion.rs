use bevy::prelude::{Component, Handle};

use crate::zmo_asset_loader::ZmoAsset;

#[derive(Component, Default)]
pub struct ActiveMotion {
    pub motion: Handle<ZmoAsset>,
    pub repeat_limit: Option<usize>, // If None, repeats forever
    pub animation_speed: f32,
    pub start_time: Option<f64>,
}

impl ActiveMotion {
    pub fn new_repeating(motion: Handle<ZmoAsset>) -> Self {
        Self {
            motion,
            repeat_limit: None,
            animation_speed: 1.0,
            start_time: None,
        }
    }

    pub fn new_once(motion: Handle<ZmoAsset>) -> Self {
        Self {
            motion,
            repeat_limit: Some(1),
            animation_speed: 1.0,
            start_time: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_repeat_n(motion: Handle<ZmoAsset>, repeat_count: usize) -> Self {
        Self {
            motion,
            repeat_limit: Some(repeat_count),
            animation_speed: 1.0,
            start_time: None,
        }
    }

    pub fn with_animation_speed(mut self, animation_speed: f32) -> Self {
        self.animation_speed = animation_speed;
        self
    }
}
