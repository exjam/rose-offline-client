use bevy::{
    core_pipeline::bloom::BloomSettings,
    prelude::{Camera, Query, ResMut},
};
use bevy_egui::{egui, EguiContexts};

use crate::{render::ZoneLighting, ui::UiStateDebugWindows};

pub fn ui_debug_zone_lighting_system(
    mut egui_context: EguiContexts,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut zone_lighting: ResMut<ZoneLighting>,
    mut query_camera: Query<(&mut Camera, &mut BloomSettings)>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Zone Lighting")
        .open(&mut ui_state_debug_windows.zone_lighting_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("zone_ambient_lighting")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Map Ambient Color:");
                    let mut map_ambient_color = [
                        zone_lighting.map_ambient_color.x,
                        zone_lighting.map_ambient_color.y,
                        zone_lighting.map_ambient_color.z,
                    ];
                    ui.color_edit_button_rgb(&mut map_ambient_color);
                    ui.end_row();

                    ui.label("Character Ambient Color:");
                    let mut character_ambient_color = [
                        zone_lighting.character_ambient_color.x,
                        zone_lighting.character_ambient_color.y,
                        zone_lighting.character_ambient_color.z,
                    ];
                    ui.color_edit_button_rgb(&mut character_ambient_color);
                    ui.end_row();

                    ui.label("Character Diffuse Color:");
                    let mut character_diffuse_color = [
                        zone_lighting.character_diffuse_color.x,
                        zone_lighting.character_diffuse_color.y,
                        zone_lighting.character_diffuse_color.z,
                    ];
                    ui.color_edit_button_rgb(&mut character_diffuse_color);
                    ui.end_row();
                });

            ui.separator();

            egui::Grid::new("zone_fog").num_columns(2).show(ui, |ui| {
                ui.label("Color Fog Enabled:");
                ui.checkbox(&mut zone_lighting.color_fog_enabled, "Enabled");
                ui.end_row();

                ui.label("Fog Color:");
                let mut fog_color = [
                    zone_lighting.fog_color.x,
                    zone_lighting.fog_color.y,
                    zone_lighting.fog_color.z,
                ];
                ui.color_edit_button_rgb(&mut fog_color);
                ui.end_row();

                ui.label("Fog Density:");
                ui.add(
                    egui::Slider::new(&mut zone_lighting.fog_density, 0.0..=0.01).show_value(true),
                );
                ui.end_row();

                ui.label("Fog Min Amount:");
                ui.add(
                    egui::Slider::new(&mut zone_lighting.fog_min_density, 0.0..=1.0)
                        .show_value(true),
                );
                ui.end_row();

                ui.label("Fog Max Amount:");
                ui.add(
                    egui::Slider::new(&mut zone_lighting.fog_max_density, 0.0..=1.0)
                        .show_value(true),
                );
                ui.end_row();
            });

            ui.separator();

            egui::Grid::new("zone_alpha_fog")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Alpha Fog Enabled:");
                    ui.checkbox(&mut zone_lighting.alpha_fog_enabled, "Enabled");
                    ui.end_row();

                    ui.label("Alpha Fog Start:");
                    ui.add(
                        egui::Slider::new(&mut zone_lighting.fog_alpha_weight_start, 0.0..=1.0)
                            .show_value(true),
                    );
                    ui.end_row();

                    ui.label("Alpha Fog End:");
                    ui.add(
                        egui::Slider::new(&mut zone_lighting.fog_alpha_weight_end, 0.0..=1.0)
                            .show_value(true),
                    );
                    ui.end_row();
                });

            ui.separator();

            if let Ok((mut camera, mut bloom_settings)) = query_camera.get_single_mut() {
                egui::Grid::new("bloom_settings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("HDR Enabled:");
                        ui.checkbox(&mut camera.hdr, "Enabled");
                        ui.end_row();
                        /*

                        /// Controls the baseline of how much the image is scattered (default: 0.15).
                        ///
                        /// This parameter should be used only to control the strength of the bloom
                        /// for the scene as a whole. Increasing it too much will make the scene appear
                        /// blurry and over-exposed.
                        ///
                        /// To make a mesh glow brighter, rather than increase the bloom intensity,
                        /// you should increase the mesh's `emissive` value.
                        ///
                        /// # In energy-conserving mode
                        /// The value represents how likely the light is to scatter.
                        ///
                        /// The value should be between 0.0 and 1.0 where:
                        /// * 0.0 means no bloom
                        /// * 1.0 means the light is scattered as much as possible
                        ///
                        /// # In additive mode
                        /// The value represents how much scattered light is added to
                        /// the image to create the glow effect.
                        ///
                        /// In this configuration:
                        /// * 0.0 means no bloom
                        /// * > 0.0 means a proportionate amount of scattered light is added
                        pub intensity: f32,

                        /// Low frequency contribution boost.
                        /// Controls how much more likely the light
                        /// is to scatter completely sideways (low frequency image).
                        ///
                        /// Comparable to a low shelf boost on an equalizer.
                        ///
                        /// # In energy-conserving mode
                        /// The value should be between 0.0 and 1.0 where:
                        /// * 0.0 means low frequency light uses base intensity for blend factor calculation
                        /// * 1.0 means low frequency light contributes at full power
                        ///
                        /// # In additive mode
                        /// The value represents how much scattered light is added to
                        /// the image to create the glow effect.
                        ///
                        /// In this configuration:
                        /// * 0.0 means no bloom
                        /// * > 0.0 means a proportionate amount of scattered light is added
                        pub low_frequency_boost: f32,

                        /// Low frequency contribution boost curve.
                        /// Controls the curvature of the blend factor function
                        /// making frequencies next to the lowest ones contribute more.
                        ///
                        /// Somewhat comparable to the Q factor of an equalizer node.
                        ///
                        /// Valid range:
                        /// * 0.0 - base base intensity and boosted intensity are linearly interpolated
                        /// * 1.0 - all frequencies below maximum are at boosted intensity level
                        pub low_frequency_boost_curvature: f32,

                        /// Tightens how much the light scatters (default: 1.0).
                        ///
                        /// Valid range:
                        /// * 0.0 - maximum scattering angle is 0 degrees (no scattering)
                        /// * 1.0 - maximum scattering angle is 90 degrees
                        pub high_pass_frequency: f32,

                        pub prefilter_settings: BloomPrefilterSettings,

                        /// Controls whether bloom textures
                        /// are blended between or added to each other. Useful
                        /// if image brightening is desired and a must-change
                        /// if `prefilter_settings` are used.
                        ///
                        /// # Recommendation
                        /// Set to [`BloomCompositeMode::Additive`] if `prefilter_settings` are
                        /// configured in a non-energy-conserving way,
                        /// otherwise set to [`BloomCompositeMode::EnergyConserving`].
                        pub composite_mode: BloomCompositeMode,
                         */

                        ui.label("Intensity:");
                        ui.add(
                            egui::Slider::new(&mut bloom_settings.intensity, 0.0..=1.0)
                                .show_value(true),
                        );
                        ui.end_row();
                    });
            }
        });
}
