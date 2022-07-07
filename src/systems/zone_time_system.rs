use bevy::{
    ecs::prelude::{Res, ResMut},
    hierarchy::Children,
    math::{const_vec3, Vec3, Vec4Swizzles},
    prelude::{Entity, Query, Visibility, With},
};

use rose_data::{SkyboxState, WORLD_TICK_DURATION};

use crate::{
    components::NightTimeEffect,
    render::ZoneLighting,
    resources::{CurrentZone, GameData, WorldTime, ZoneTime, ZoneTimeState},
};

const MORNING_FOG_COLOR: Vec3 = const_vec3!([100.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0]);
const MORNING_FOG_DENSITY: f32 = 0.0022;

const DAY_FOG_COLOR: Vec3 = const_vec3!([200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0]);
const DAY_FOG_DENSITY: f32 = 0.0018;

const EVENING_FOG_COLOR: Vec3 = const_vec3!([100.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0]);
const EVENING_FOG_DENSITY: f32 = 0.0022;

const NIGHT_FOG_COLOR: Vec3 = const_vec3!([10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0]);
const NIGHT_FOG_DENSITY: f32 = 0.0020;

fn set_visible_recursive(
    is_visible: bool,
    entity: Entity,
    query_visibility: &mut Query<&mut Visibility>,
    query_children: &Query<&Children>,
) {
    if let Ok(mut visibility) = query_visibility.get_mut(entity) {
        visibility.is_visible = is_visible;
    }

    if let Ok(children) = query_children.get(entity) {
        for child in children.iter() {
            set_visible_recursive(is_visible, *child, query_visibility, query_children);
        }
    }
}

pub trait SingleLerp {
    fn lerp(self, end: Self, s: f32) -> Self;
}

impl SingleLerp for f32 {
    fn lerp(self, end: Self, s: f32) -> Self {
        self * (1.0 - s) + end * s
    }
}

pub fn zone_time_system(
    mut zone_lighting: ResMut<ZoneLighting>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    world_time: Res<WorldTime>,
    mut zone_time: ResMut<ZoneTime>,
    mut query_night_effects: Query<Entity, With<NightTimeEffect>>,
    mut query_visibility: Query<&mut Visibility>,
    query_children: Query<&Children>,
) {
    if current_zone.is_none() {
        return;
    }
    let current_zone = current_zone.unwrap();
    let zone_data = game_data.zone_list.get_zone(current_zone.id);
    if zone_data.is_none() {
        return;
    }
    let zone_data = zone_data.unwrap();
    let skybox_data = zone_data
        .skybox_id
        .and_then(|id| game_data.skybox.get_skybox_data(id));

    let world_day_time = world_time.ticks.get_world_time();
    let (day_time, partial_tick) = if let Some(overwrite_time) = zone_time.debug_overwrite_time {
        (overwrite_time, 0.0)
    } else {
        (
            world_day_time % zone_data.day_cycle,
            world_time.time_since_last_tick.as_secs_f32() / WORLD_TICK_DURATION.as_secs_f32(),
        )
    };

    if day_time >= zone_data.night_time || day_time < zone_data.morning_time {
        let state_length = zone_data.morning_time + (zone_data.day_cycle - zone_data.night_time);
        let state_ticks = day_time - zone_data.night_time;

        if zone_time.state != ZoneTimeState::Night {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(true, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Night;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        if let Some(skybox_data) = skybox_data {
            zone_lighting.map_ambient_color =
                skybox_data.map_ambient_color[SkyboxState::Night].xyz();
            zone_lighting.character_ambient_color =
                skybox_data.character_ambient_color[SkyboxState::Night].xyz();
            zone_lighting.character_diffuse_color =
                skybox_data.character_diffuse_color[SkyboxState::Night].xyz();
            zone_lighting.fog_color = NIGHT_FOG_COLOR;
            zone_lighting.fog_density = NIGHT_FOG_DENSITY;
        }
    } else if day_time >= zone_data.evening_time {
        let state_length = zone_data.night_time - zone_data.evening_time;
        let state_ticks = day_time - zone_data.evening_time;

        if zone_time.state != ZoneTimeState::Evening {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(true, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Evening;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        if let Some(skybox_data) = skybox_data {
            if zone_time.state_percent_complete < 0.5 {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color[SkyboxState::Day]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Evening],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Day]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Evening],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Day]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Evening],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color =
                    DAY_FOG_COLOR.lerp(EVENING_FOG_COLOR, zone_time.state_percent_complete * 2.0);
                zone_lighting.fog_density = DAY_FOG_DENSITY
                    .lerp(EVENING_FOG_DENSITY, zone_time.state_percent_complete * 2.0);
            } else {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color
                    [SkyboxState::Evening]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Night],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Evening]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Night],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Evening]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Night],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color = EVENING_FOG_COLOR.lerp(
                    NIGHT_FOG_COLOR,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
                zone_lighting.fog_density = EVENING_FOG_DENSITY.lerp(
                    NIGHT_FOG_DENSITY,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
            }
        }
    } else if day_time >= zone_data.day_time {
        let state_length = zone_data.evening_time - zone_data.day_time;
        let state_ticks = day_time - zone_data.day_time;

        if zone_time.state != ZoneTimeState::Day {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(false, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Day;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        if let Some(skybox_data) = skybox_data {
            zone_lighting.map_ambient_color = skybox_data.map_ambient_color[SkyboxState::Day].xyz();
            zone_lighting.character_ambient_color =
                skybox_data.character_ambient_color[SkyboxState::Day].xyz();
            zone_lighting.character_diffuse_color =
                skybox_data.character_diffuse_color[SkyboxState::Day].xyz();
            zone_lighting.fog_color = DAY_FOG_COLOR;
            zone_lighting.fog_density = DAY_FOG_DENSITY;
        }
    } else if day_time >= zone_data.morning_time {
        let state_length = zone_data.day_time - zone_data.morning_time;
        let state_ticks = day_time - zone_data.morning_time;

        if zone_time.state != ZoneTimeState::Morning {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(false, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Morning;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        if let Some(skybox_data) = skybox_data {
            if zone_time.state_percent_complete < 0.5 {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color[SkyboxState::Night]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Morning],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Night]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Morning],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Night]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Morning],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color =
                    NIGHT_FOG_COLOR.lerp(MORNING_FOG_COLOR, zone_time.state_percent_complete * 2.0);
                zone_lighting.fog_density = NIGHT_FOG_DENSITY
                    .lerp(MORNING_FOG_DENSITY, zone_time.state_percent_complete * 2.0);
            } else {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color
                    [SkyboxState::Morning]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Day],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Morning]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Day],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Morning]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Day],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color = MORNING_FOG_COLOR.lerp(
                    DAY_FOG_COLOR,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
                zone_lighting.fog_density = MORNING_FOG_DENSITY.lerp(
                    DAY_FOG_DENSITY,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
            }
        }
    }

    zone_time.time = day_time;
}
