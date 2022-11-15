use bevy::prelude::Resource;
use rand::Rng;
use std::time::Duration;

use rose_data::WorldTicks;

#[derive(Resource)]
pub struct WorldTime {
    pub ticks: WorldTicks,
    pub time_since_last_tick: Duration,
}

impl Default for WorldTime {
    fn default() -> Self {
        Self::new(WorldTicks(rand::thread_rng().gen_range(0..=9999)))
    }
}

impl WorldTime {
    pub fn new(ticks: WorldTicks) -> Self {
        Self {
            ticks,
            time_since_last_tick: Duration::from_secs(0),
        }
    }
}
