use bevy::prelude::Component;
use std::time::Duration;

#[derive(Component)]
pub struct PassiveRecoveryTime {
    pub time: Duration,
}

impl PassiveRecoveryTime {
    pub fn default() -> Self {
        Self {
            time: Duration::from_secs(0),
        }
    }
}
