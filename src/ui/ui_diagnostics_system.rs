use bevy::{
    diagnostic::Diagnostics,
    prelude::{Res, ResMut},
};
use bevy_egui::{egui, EguiContext};

pub fn ui_diagnostics_system(mut egui_context: ResMut<EguiContext>, diagnostics: Res<Diagnostics>) {
    egui::Window::new("Diagnostics")
        .vscroll(true)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("diagnostics_grid")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.label("Value");
                    ui.label("Average");
                    ui.end_row();

                    for diagnostic in diagnostics.iter() {
                        if let Some(value) = diagnostic.value() {
                            if let Some(average) = diagnostic.average() {
                                ui.label(diagnostic.name.as_ref());
                                ui.label(format!("{:>11.6}{:1}", value, diagnostic.suffix));
                                ui.label(format!("{:>.6}{}", average, diagnostic.suffix));
                                ui.end_row();
                            } else {
                                ui.label(diagnostic.name.as_ref());
                                ui.label(format!("{:>11.6}{:1}", value, diagnostic.suffix));
                                ui.end_row();
                            }
                        }
                    }
                });
        });
}
