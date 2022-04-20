use bevy::prelude::{Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};
use rose_data::WORLD_TICK_DURATION;

use crate::{
    resources::{CurrentZone, GameData, WorldTime, ZoneTime},
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugZoneTime {
    pub overwrite_time_enabled: bool,
    pub overwrite_time_value: u32,
}

pub fn ui_debug_zone_time_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_zone_time: Local<UiStateDebugZoneTime>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    world_time: Res<WorldTime>,
    mut zone_time: ResMut<ZoneTime>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    if current_zone.is_none() {
        return;
    }
    let current_zone = current_zone.unwrap();
    let zone_data = game_data.zone_list.get_zone(current_zone.id);
    if zone_data.is_none() {
        return;
    }
    let zone_data = zone_data.unwrap();

    egui::Window::new("Zone Time")
        .open(&mut ui_state_debug_windows.zone_time_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("zone_time_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("World Time:");
                    ui.label(format!(
                        "{:?}.{:03}",
                        world_time.ticks.0,
                        (1000.0
                            * (world_time.time_since_last_tick.as_secs_f32()
                                / WORLD_TICK_DURATION.as_secs_f32()))
                            as i32
                    ));
                    ui.end_row();

                    ui.label("Zone Time:");
                    ui.label(format!("{}", zone_time.time));
                    ui.end_row();

                    ui.checkbox(
                        &mut ui_state_debug_zone_time.overwrite_time_enabled,
                        "Overwrite Time",
                    );
                    ui.add(egui::Slider::new(
                        &mut ui_state_debug_zone_time.overwrite_time_value,
                        0..=zone_data.day_cycle,
                    ));
                    ui.end_row();

                    if ui_state_debug_zone_time.overwrite_time_enabled {
                        zone_time.debug_overwrite_time =
                            Some(ui_state_debug_zone_time.overwrite_time_value);
                    } else {
                        zone_time.debug_overwrite_time = None;
                    }
                });

            ui.separator();

            egui::Grid::new("zone_time_state_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("State:");
                    ui.label(format!("{:?}", zone_time.state));
                    ui.end_row();

                    ui.label("State blend weight:");
                    ui.label(format!("{:.3}", zone_time.state_percent_complete));
                    ui.end_row();
                });

            ui.separator();

            egui::Grid::new("zone_time_cycle_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Day Cycle:");
                    ui.label(format!("{}", zone_data.day_cycle));
                    ui.end_row();

                    ui.label("Morning Time:");
                    ui.label(format!("{}", zone_data.morning_time));
                    ui.end_row();

                    ui.label("Day Time:");
                    ui.label(format!("{}", zone_data.day_time));
                    ui.end_row();

                    ui.label("Evening Time:");
                    ui.label(format!("{}", zone_data.evening_time));
                    ui.end_row();

                    ui.label("Night Time:");
                    ui.label(format!("{}", zone_data.night_time));
                    ui.end_row();
                });
        });
}
