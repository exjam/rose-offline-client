use bevy::ecs::prelude::{Res, ResMut};
use rose_data::WORLD_TICK_DURATION;

use crate::resources::{WorldTime, ZoneTime, ZoneTimeState};

pub fn zone_time_system(world_time: Res<WorldTime>, mut zone_time: ResMut<ZoneTime>) {
    let world_day_time = world_time.ticks.get_world_time();
    let (day_time, partial_tick) = if let Some(overwrite_time) = zone_time.debug_overwrite_time {
        (overwrite_time, 0.0)
    } else {
        (
            world_day_time % zone_time.day_cycle,
            world_time.time_since_last_tick.as_secs_f32() / WORLD_TICK_DURATION.as_secs_f32(),
        )
    };

    if day_time >= zone_time.night_time || day_time < zone_time.morning_time {
        let state_length = zone_time.morning_time + (zone_time.day_cycle - zone_time.night_time);
        let state_ticks = day_time - zone_time.night_time;
        zone_time.state = ZoneTimeState::Night;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;
    } else if day_time >= zone_time.evening_time {
        let state_length = zone_time.night_time - zone_time.evening_time;
        let state_ticks = day_time - zone_time.evening_time;
        zone_time.state = ZoneTimeState::Evening;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;
    } else if day_time >= zone_time.day_time {
        let state_length = zone_time.evening_time - zone_time.day_time;
        let state_ticks = day_time - zone_time.day_time;
        zone_time.state = ZoneTimeState::Day;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;
    } else if day_time >= zone_time.morning_time {
        let state_length = zone_time.day_time - zone_time.morning_time;
        let state_ticks = day_time - zone_time.morning_time;
        zone_time.state = ZoneTimeState::Morning;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;
    }

    zone_time.time = day_time;
}
