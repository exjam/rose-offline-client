use bevy::prelude::{Local, Query, ResMut};
use bevy_egui::{egui, EguiContexts};

use crate::{
    audio::SoundGain, components::SoundCategory, resources::SoundSettings, ui::UiStateWindows,
};

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
    mut sound_settings: ResMut<SoundSettings>,
    mut query_sounds: Query<(&SoundCategory, &mut SoundGain)>,
) {
    egui::Window::new("Settings")
        .open(&mut ui_state_windows.settings_open)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sound, "Sound");
            });

            egui::Grid::new("sound_settings_gain")
                .num_columns(2)
                .show(ui, |ui| {
                    let mut gain_changed = false;

                    ui.label("Sound:");
                    gain_changed |= ui
                        .checkbox(&mut sound_settings.enabled, "Enabled")
                        .changed();
                    ui.end_row();

                    ui.label("Global Volume:");
                    gain_changed |= ui
                        .add(
                            egui::Slider::new(&mut sound_settings.global_gain, 0.0..=1.0)
                                .show_value(true),
                        )
                        .changed();
                    ui.end_row();

                    let mut add_category_slider = |text: &str, category| {
                        ui.label(text);
                        gain_changed |= ui
                            .add(
                                egui::Slider::new(&mut sound_settings.gains[category], 0.0..=1.0)
                                    .show_value(true),
                            )
                            .changed();
                        ui.end_row();
                    };

                    add_category_slider("Background Music:", SoundCategory::BackgroundMusic);
                    add_category_slider("Player Footsteps:", SoundCategory::PlayerFootstep);
                    add_category_slider("Other Footsteps:", SoundCategory::OtherFootstep);
                    add_category_slider("Player Combat:", SoundCategory::PlayerCombat);
                    add_category_slider("Other Combat:", SoundCategory::OtherCombat);
                    add_category_slider("NPC Sounds:", SoundCategory::NpcSounds);

                    if gain_changed {
                        for (category, mut gain) in query_sounds.iter_mut() {
                            let target_gain = sound_settings.gain(*category);

                            if target_gain != *gain {
                                *gain = target_gain;
                            }
                        }
                    }
                });
        });
}
