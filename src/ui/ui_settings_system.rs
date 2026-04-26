use crate::{
    audio::SoundGain,
    components::{NameTagType, SoundCategory},
    resources::{NameTagCache, TargetingType},
    save_config,
    ui::UiStateWindows,
    Config, GraphicsModeConfig,
};
use bevy::{
    prelude::{Local, Query, ResMut},
    window::WindowMode,
};
use bevy_egui::{egui, EguiContexts};
use egui::{vec2, KeyboardShortcut, Ui};
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Debug)]
enum SettingsPage {
    Video,
    Sound,
    Interface,
    Hotkeys,
}

pub struct UiStateSettings {
    page: SettingsPage,
}

impl Default for UiStateSettings {
    fn default() -> Self {
        Self {
            page: SettingsPage::Video,
        }
    }
}

pub fn ui_settings_system(
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_state_settings: Local<UiStateSettings>,
    mut config: ResMut<Config>,
    mut query_sounds: Query<(&SoundCategory, &mut SoundGain)>,
    mut name_tag_cache: ResMut<NameTagCache>,
) {
    egui::Window::new("Settings")
        .open(&mut ui_state_windows.settings_open)
        .resizable(false)
        .fixed_size(vec2(200.0, 200.0))
        .show(egui_context.ctx_mut(), |ui| {
            let mut save_settings = false;

            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Video, "Video");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sound, "Sound");
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Interface,
                    "Interface",
                );

                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Hotkeys,
                    "Key Bindings",
                );
            });

            match ui_state_settings.page {
                SettingsPage::Video => {
                    egui::Grid::new("video_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            let mut mode_changed = false;
                            let graphics = &mut config.graphics;

                            ui.label("Screen mode");

                            let mut selected_mode =
                                if graphics.mode == GraphicsModeConfig::Fullscreen {
                                    WindowMode::Fullscreen
                                } else {
                                    WindowMode::Windowed
                                };

                            egui::ComboBox::from_id_source("screen_mode")
                                .selected_text(format!("{:?}", selected_mode))
                                .show_ui(ui, |ui| {
                                    mode_changed |= ui
                                        .selectable_value(
                                            &mut selected_mode,
                                            WindowMode::Windowed,
                                            format!("{:?}", WindowMode::Windowed),
                                        )
                                        .changed();

                                    mode_changed |= ui
                                        .selectable_value(
                                            &mut selected_mode,
                                            WindowMode::Fullscreen,
                                            format!("{:?}", WindowMode::Fullscreen),
                                        )
                                        .changed();
                                });

                            ui.end_row();
                            ui.label("Resolution");

                            ui.add_enabled_ui(selected_mode == WindowMode::Windowed, |ui| {
                                egui::ComboBox::from_id_source("resolution")
                                    .selected_text(match graphics.mode {
                                        GraphicsModeConfig::Window { width, height } => {
                                            format!("{}x{}", width, height)
                                        }
                                        GraphicsModeConfig::Fullscreen => "".to_string(),
                                    })
                                    .show_ui(ui, |ui| {
                                        for resolution in &graphics.resolutions {
                                            let (width, height) = resolution;

                                            save_settings |= ui
                                                .selectable_value(
                                                    &mut graphics.mode,
                                                    GraphicsModeConfig::Window {
                                                        width: width.clone() as f32,
                                                        height: height.clone() as f32,
                                                    },
                                                    format!("{}x{}", width, height),
                                                )
                                                .changed();
                                        }
                                    });
                            });

                            if mode_changed {
                                match selected_mode {
                                    WindowMode::Windowed => {
                                        let (width, height) =
                                            graphics.resolutions.last().unwrap_or(&(1920, 1080));

                                        config.graphics.mode = GraphicsModeConfig::Window {
                                            width: width.clone() as f32,
                                            height: height.clone() as f32,
                                        }
                                    }
                                    WindowMode::Fullscreen => {
                                        config.graphics.mode = GraphicsModeConfig::Fullscreen
                                    }
                                    _ => {}
                                }

                                save_settings = true;
                            }
                        });
                }
                SettingsPage::Sound => {
                    egui::Grid::new("sound_settings_gain")
                        .num_columns(2)
                        .show(ui, |ui| {
                            let mut gain_changed = false;

                            ui.label("Sound");
                            save_settings |=
                                ui.checkbox(&mut config.sound.enabled, "Enabled").changed();
                            ui.end_row();

                            ui.label("Global Volume");
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
                                save_settings |= category_response.drag_released()
                                    || category_response.lost_focus();
                                ui.end_row();
                            };

                            let volume = &mut config.sound.volume;
                            add_category_slider("Background Music", &mut volume.background_music);
                            add_category_slider("Player Footsteps", &mut volume.player_footstep);
                            add_category_slider("Other Footsteps", &mut volume.other_footstep);
                            add_category_slider("Player Combat", &mut volume.player_combat);
                            add_category_slider("Other Combat", &mut volume.other_combat);
                            add_category_slider("NPC Sounds", &mut volume.npc_sounds);

                            if gain_changed || save_settings {
                                for (category, mut gain) in query_sounds.iter_mut() {
                                    let target_gain = config.sound.gain(*category);

                                    if target_gain != *gain {
                                        *gain = target_gain;
                                    }
                                }
                            }
                        });
                }
                SettingsPage::Interface => {
                    egui::Grid::new("interface_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Control");
                            save_settings |= ui
                                .radio_value(
                                    &mut config.interface.targeting,
                                    TargetingType::DoubleClick,
                                    "One Click: Target\nDouble Click: Attack",
                                )
                                .changed();
                            ui.end_row();

                            ui.label("");
                            save_settings |= ui
                                .radio_value(
                                    &mut config.interface.targeting,
                                    TargetingType::SingleClick,
                                    "One Click: Target + Attack",
                                )
                                .changed();
                            ui.end_row();

                            ui.end_row();
                            let mut name_tag_changed = false;

                            ui.label("Info");
                            name_tag_changed |= ui
                                .checkbox(
                                    &mut config.interface.name_tag_settings.show_all
                                        [NameTagType::Character],
                                    "Other Player Name",
                                )
                                .changed();
                            ui.end_row();

                            ui.label("");
                            name_tag_changed |= ui
                                .checkbox(
                                    &mut config.interface.name_tag_settings.show_all
                                        [NameTagType::Npc],
                                    "NPC Name",
                                )
                                .changed();
                            ui.end_row();

                            ui.label("");
                            name_tag_changed |= ui
                                .checkbox(
                                    &mut config.interface.name_tag_settings.show_all
                                        [NameTagType::Monster],
                                    "Monster Name",
                                )
                                .changed();

                            if name_tag_changed {
                                save_settings = true;
                                name_tag_cache.dispose = true;
                            }
                        });
                }
                SettingsPage::Hotkeys => {
                    egui::Grid::new("hotkey_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            let mut add_shortcut_setting =
                                |ui: &mut Ui, text: &str, value: &mut KeyboardShortcut| {
                                    ui.label(text);

                                    let mut shortcut_label =
                                        ui.ctx().format_shortcut(&value.clone());
                                    let response =
                                        ui.add(egui::TextEdit::singleline(&mut shortcut_label));

                                    if response.has_focus() {
                                        let shortcut_option = ui.input_mut(|state| {
                                            if state.keys_down.is_empty() {
                                                return None;
                                            }

                                            let key = state.keys_down.iter().next()?;
                                            let shortcut =
                                                KeyboardShortcut::new(state.modifiers, *key);
                                            state.consume_shortcut(&shortcut);

                                            Some(shortcut)
                                        });

                                        if let Some(new_shortcut) = shortcut_option {
                                            response.surrender_focus();
                                            *value = new_shortcut;
                                        }
                                    }

                                    if response.lost_focus() {
                                        save_settings = true;
                                    }

                                    ui.end_row();
                                };

                            add_shortcut_setting(ui, "Inventory", &mut config.hotkeys.inventory);
                            add_shortcut_setting(ui, "Skills", &mut config.hotkeys.skills);
                            add_shortcut_setting(ui, "Character", &mut config.hotkeys.character);
                            add_shortcut_setting(ui, "Quest Log", &mut config.hotkeys.quests);
                            add_shortcut_setting(ui, "Clan", &mut config.hotkeys.clan);
                            add_shortcut_setting(ui, "Settings", &mut config.hotkeys.settings);
                            add_shortcut_setting(ui, "Exit", &mut config.hotkeys.exit);

                            ui.end_row();

                            add_shortcut_setting(ui, "Hotbar Slot 1", &mut config.hotkeys.hotbar_1);
                            add_shortcut_setting(ui, "Hotbar Slot 2", &mut config.hotkeys.hotbar_2);
                            add_shortcut_setting(ui, "Hotbar Slot 3", &mut config.hotkeys.hotbar_3);
                            add_shortcut_setting(ui, "Hotbar Slot 4", &mut config.hotkeys.hotbar_4);
                            add_shortcut_setting(ui, "Hotbar Slot 5", &mut config.hotkeys.hotbar_5);
                            add_shortcut_setting(ui, "Hotbar Slot 6", &mut config.hotkeys.hotbar_6);
                            add_shortcut_setting(ui, "Hotbar Slot 7", &mut config.hotkeys.hotbar_7);
                            add_shortcut_setting(ui, "Hotbar Slot 8", &mut config.hotkeys.hotbar_8);
                        });
                }
            };

            if !save_settings {
                return;
            }

            let path = config.filesystem.config_path.clone();
            save_config(config.into_inner(), Path::new(&path));
        });
}
