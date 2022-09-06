use bevy::{
    asset::HandleId,
    prelude::{AssetServer, Assets, Handle, Local, Res, ResMut},
};
use bevy_egui::{egui, EguiContext};
use regex::Regex;

use crate::ui::{
    widgets::{DataBindings, Dialog},
    UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugDialogs {
    draw_dialog: Option<Handle<Dialog>>,
    filter_name: String,
    filtered_dialogs: Vec<(String, HandleId)>,
}

pub fn ui_debug_dialog_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state: Local<UiStateDebugDialogs>,
    asset_server: Res<AssetServer>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state = &mut *ui_state;
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Dialog List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.dialog_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut filter_changed = false;

            egui::Grid::new("dialog_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Dialog Path Filter:");
                    if ui.text_edit_singleline(&mut ui_state.filter_name).changed() {
                        filter_changed = true;
                    }
                    ui.end_row();
                });

            if ui_state.filter_name.is_empty() && ui_state.filtered_dialogs.is_empty() {
                filter_changed = true;
            }

            if filter_changed {
                let filter_name_re = if !ui_state.filter_name.is_empty() {
                    Some(
                        Regex::new(&format!("(?i){}", regex::escape(&ui_state.filter_name)))
                            .unwrap(),
                    )
                } else {
                    None
                };

                ui_state.filtered_dialogs = dialog_assets
                    .iter()
                    .filter_map(|(handle_id, _)| {
                        let asset_path = asset_server
                            .get_handle_path(handle_id)?
                            .path()
                            .to_string_lossy()
                            .to_string();
                        if !filter_name_re
                            .as_ref()
                            .map_or(true, |re| re.is_match(&asset_path))
                        {
                            None
                        } else {
                            Some((asset_path, handle_id))
                        }
                    })
                    .collect();
                ui_state.filtered_dialogs.sort_by(|(a, _), (b, _)| a.cmp(b));
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::remainder().at_least(80.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(
                        20.0,
                        ui_state.filtered_dialogs.len(),
                        |row_index, mut row| {
                            let (path, handle_id) = &ui_state.filtered_dialogs[row_index];
                            row.col(|ui| {
                                ui.label(path);
                            });

                            row.col(|ui| {
                                if ui.button("View").clicked() {
                                    ui_state.draw_dialog = Some(Handle::weak(*handle_id));
                                }
                            });
                        },
                    );
                });
        });

    if !ui_state_debug_windows.dialog_list_open {
        return;
    }

    if let Some(dialog) = ui_state
        .draw_dialog
        .as_ref()
        .and_then(|handle| dialog_assets.get(handle))
    {
        egui::Window::new("DebugDialogViewer")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(egui::Frame::none())
            .title_bar(false)
            .resizable(false)
            .default_width(dialog.width)
            .default_height(dialog.height)
            .show(egui_context.ctx_mut(), |ui| {
                dialog.draw(ui, DataBindings::default(), |_, _| {})
            });
    }
}
