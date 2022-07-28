use bevy::prelude::{AssetServer, Assets, Handle, Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};

use crate::ui::{draw_dialog, Dialog, DialogDataBindings, UiStateDebugWindows};

#[derive(Default)]
pub struct UiStateDebugDialogs {
    pub draw_dialog: Option<Handle<Dialog>>,
}

pub fn ui_debug_dialog_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_dialogs: Local<UiStateDebugDialogs>,
    asset_server: Res<AssetServer>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Dialog List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.dialog_list_open)
        .show(egui_context.ctx_mut(), |ui| {
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
                .body(|mut body| {
                    for (handle_id, _) in dialog_assets.iter() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                if let Some(asset_path) = asset_server.get_handle_path(handle_id) {
                                    if let Some(str) = asset_path.path().to_str() {
                                        ui.label(str);
                                    }
                                }
                            });

                            row.col(|ui| {
                                if ui.button("View").clicked() {
                                    ui_state_debug_dialogs.draw_dialog =
                                        Some(Handle::weak(handle_id));
                                }
                            });
                        });
                    }
                });
        });

    if !ui_state_debug_windows.dialog_list_open {
        return;
    }

    if let Some(dialog) = ui_state_debug_dialogs
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
                draw_dialog(ui, dialog, DialogDataBindings::default(), |_, _| {})
            });
    }
}
