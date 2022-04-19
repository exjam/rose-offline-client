use std::time::Duration;

use bevy::{
    core::Time,
    ecs::prelude::{Res, ResMut},
};

use rose_data::{WorldTicks, WORLD_TICK_DURATION};

use crate::resources::WorldTime;

pub fn world_time_system(time: Res<Time>, mut world_time: ResMut<WorldTime>) {
    world_time.time_since_last_tick += Duration::from_secs_f64(time.delta_seconds_f64());

    if world_time.time_since_last_tick > WORLD_TICK_DURATION {
        world_time.ticks = world_time.ticks + WorldTicks(1);
        world_time.time_since_last_tick -= WORLD_TICK_DURATION;
    }
}
