use rand::Rng;
use std::time::Duration;

use rose_data::WorldTicks;

pub struct WorldTime {
    pub ticks: WorldTicks,
    pub time_since_last_tick: Duration,
}

impl WorldTime {
    pub fn default() -> Self {
        Self::new(WorldTicks(rand::thread_rng().gen_range(0..=9999)))
    }

    pub fn new(ticks: WorldTicks) -> Self {
        Self {
            ticks,
            time_since_last_tick: Duration::from_secs(0),
        }
    }
}
