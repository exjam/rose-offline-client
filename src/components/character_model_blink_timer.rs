use std::ops::Range;

use bevy::{prelude::Component, reflect::Reflect};
use rand::Rng;

#[derive(Component, Reflect)]
pub struct CharacterBlinkTimer {
    pub timer: f32,
    pub is_open: bool,
    pub closed_duration: f32,
    pub open_duration: f32,
}

impl CharacterBlinkTimer {
    pub const BLINK_CLOSED_DURATION: Range<f32> = 0.010..0.110;
    pub const BLINK_OPEN_DURATION: Range<f32> = 0.100..3.000;

    pub fn new() -> Self {
        Self {
            timer: 0.0,
            is_open: false,
            closed_duration: rand::thread_rng().gen_range(Self::BLINK_CLOSED_DURATION),
            open_duration: rand::thread_rng().gen_range(Self::BLINK_OPEN_DURATION),
        }
    }
}

impl Default for CharacterBlinkTimer {
    fn default() -> Self {
        Self::new()
    }
}
