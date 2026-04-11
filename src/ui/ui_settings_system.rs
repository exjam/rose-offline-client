use bevy::prelude::{Local, Query, ResMut};
use bevy_egui::{egui, EguiContexts};
use std::path::Path;

use crate::{audio::SoundGain, components::SoundCategory, save_config, ui::UiStateWindows, Config};

#[derive(Copy, Clone, PartialEq, Debug)]
enum SettingsPage {
    Sound,
}

pub struct UiStateSettings {
    page: SettingsPage,
}

impl Default for UiStateSettings {
    fn default() -> Self {
        Self {
            page: SettingsPage::Sound,
        }
    }
}

pub fn ui_settings_system(
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_state_settings: Local<UiStateSettings>,
    mut config: ResMut<Config>,
    mut query_sounds: Query<(&SoundCategory, &mut SoundGain)>,
) {
    egui::Window::new("Settings")
        .open(&mut ui_state_windows.settings_open)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            let mut save_settings = false;

            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sound, "Sound");
            });

            egui::Grid::new("sound_settings_gain")
                .num_columns(2)
                .show(ui, |ui| {
                    let mut gain_changed = false;

                    ui.label("Sound:");
                    save_settings |= ui.checkbox(&mut config.sound.enabled, "Enabled").changed();
                    ui.end_row();

                    ui.label("Global Volume:");
                    let global_response = ui.add(
                        egui::Slider::new(&mut config.sound.volume.global, 0.0..=1.0)
                            .show_value(true),
                    );
                    gain_changed |= global_response.changed();
                    save_settings |=
                        global_response.drag_released() || global_response.lost_focus();
                    ui.end_row();

                    let mut add_category_slider = |text: &str, value| {
                        ui.label(text);
                        let category_response =
                            ui.add(egui::Slider::new(value, 0.0..=1.0).show_value(true));
                        gain_changed |= category_response.changed();
                        save_settings |=
                            category_response.drag_released() || category_response.lost_focus();
                        ui.end_row();
                    };

                    let volume = &mut config.sound.volume;
                    add_category_slider("Background Music:", &mut volume.background_music);
                    add_category_slider("Player Footsteps:", &mut volume.player_footstep);
                    add_category_slider("Other Footsteps:", &mut volume.other_footstep);
                    add_category_slider("Player Combat:", &mut volume.player_combat);
                    add_category_slider("Other Combat:", &mut volume.other_combat);
                    add_category_slider("NPC Sounds:", &mut volume.npc_sounds);

                    if gain_changed || save_settings {
                        for (category, mut gain) in query_sounds.iter_mut() {
                            let target_gain = config.sound.gain(*category);

                            if target_gain != *gain {
                                *gain = target_gain;
                            }
                        }
                    }
                });

            if !save_settings {
                return;
            }

            let path = config.filesystem.config_path.clone();
            save_config(config.into_inner(), Path::new(&path));
        });
}
