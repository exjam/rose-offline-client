use std::time::Duration;

use bevy::{prelude::Component, reflect::Reflect};

#[derive(Component, Default, Reflect)]
pub struct PassiveRecoveryTime {
    pub time: Duration,
}
