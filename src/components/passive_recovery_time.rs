use bevy::prelude::Component;
use std::time::Duration;

#[derive(Component, Default)]
pub struct PassiveRecoveryTime {
    pub time: Duration,
}
