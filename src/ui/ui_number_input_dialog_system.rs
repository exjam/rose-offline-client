use bevy::prelude::{Assets, Commands, Events, Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};

use crate::{
    events::NumberInputDialogEvent,
    resources::UiResources,
    ui::{widgets::Dialog, DataBindings},
};

const IID_EDITBOX: i32 = 2;
const IID_BUTTON_CLOSE: i32 = 3;
const IID_BTN_DEL: i32 = 4;
const IID_BTN_OK: i32 = 5;
const IID_BTN_MAX: i32 = 6;
const IID_BTN_0: i32 = 10;
const IID_BTN_1: i32 = 11;
const IID_BTN_2: i32 = 12;
const IID_BTN_3: i32 = 13;
const IID_BTN_4: i32 = 14;
const IID_BTN_5: i32 = 15;
const IID_BTN_6: i32 = 16;
const IID_BTN_7: i32 = 17;
const IID_BTN_8: i32 = 18;
const IID_BTN_9: i32 = 19;

pub struct ActiveNumberInputDialog {
    current_value: String,
    has_set_position: bool,
    max_value: Option<usize>,
    modal: bool,
    ok: Option<Box<dyn FnOnce(&mut Commands, usize) + Send + Sync>>,
    cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
}

#[derive(Default)]
pub struct UiStateMessageBox {
    active: Option<ActiveNumberInputDialog>,
}

pub fn ui_number_input_dialog_system(
    mut commands: Commands,
    mut ui_state: Local<UiStateMessageBox>,
    mut egui_context: ResMut<EguiContext>,
    mut number_input_dialog_events: ResMut<Events<NumberInputDialogEvent>>,
    dialog_assets: Res<Assets<Dialog>>,
    ui_resources: Res<UiResources>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_number_input) {
        dialog
    } else {
        return;
    };
    for event in number_input_dialog_events.drain() {
        let NumberInputDialogEvent::Show {
            max_value,
            modal,
            ok,
            cancel,
        } = event;

        // Cancel any currently open dialog
        if let Some(mut active) = ui_state.active.take() {
            if let Some(cancel) = active.cancel.take() {
                cancel(&mut commands);
            }
        }

        ui_state.active = Some(ActiveNumberInputDialog {
            current_value: String::with_capacity(32),
            has_set_position: false,
            max_value,
            modal,
            ok,
            cancel,
        });
    }

    if ui_state.active.as_ref().map_or(false, |x| x.modal) {
        egui::Area::new("modal_ninput")
            .interactable(true)
            .fixed_pos(egui::Pos2::ZERO)
            .show(egui_context.ctx_mut(), |ui| {
                let interceptor_rect = ui.ctx().input().screen_rect();

                ui.allocate_response(interceptor_rect.size(), egui::Sense::click_and_drag());
                ui.allocate_ui_at_rect(interceptor_rect, |ui| {
                    ui.painter().add(egui::epaint::Shape::rect_filled(
                        interceptor_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 144),
                    ));
                });
            });
    }

    let active_dialog = if let Some(active_dialog) = ui_state.active.as_mut() {
        active_dialog
    } else {
        return;
    };

    let screen_size = egui_context.ctx_mut().input().screen_rect().size();
    let default_x = screen_size.x / 2.0 - dialog.width / 2.0;
    let default_y = screen_size.y / 2.0 - dialog.height / 2.0;

    let mut response_button_ok = None;
    let mut response_button_cancel = None;
    let mut response_button_del = None;
    let mut response_button_max = None;
    let mut response_button_0 = None;
    let mut response_button_1 = None;
    let mut response_button_2 = None;
    let mut response_button_3 = None;
    let mut response_button_4 = None;
    let mut response_button_5 = None;
    let mut response_button_6 = None;
    let mut response_button_7 = None;
    let mut response_button_8 = None;
    let mut response_button_9 = None;
    let mut response_editbox = None;

    let mut area = egui::Area::new("num_input_dlg")
        .movable(true)
        .interactable(true)
        .default_pos([default_x, default_y])
        .order(egui::Order::Foreground);

    let first_show = if !active_dialog.has_set_position {
        area = area.current_pos([default_x, default_y]);
        active_dialog.has_set_position = true;
        true
    } else {
        false
    };

    let response = area.show(egui_context.ctx_mut(), |ui| {
        let response = ui.allocate_response(
            egui::vec2(dialog.width, dialog.height),
            egui::Sense::hover(),
        );

        dialog.draw(
            ui,
            DataBindings {
                visible: &mut [(IID_BTN_MAX, active_dialog.max_value.is_some())],
                text: &mut [(IID_EDITBOX, &mut active_dialog.current_value)],
                response: &mut [
                    (IID_BTN_OK, &mut response_button_ok),
                    (IID_BUTTON_CLOSE, &mut response_button_cancel),
                    (IID_BTN_DEL, &mut response_button_del),
                    (IID_BTN_MAX, &mut response_button_max),
                    (IID_BTN_0, &mut response_button_0),
                    (IID_BTN_1, &mut response_button_1),
                    (IID_BTN_2, &mut response_button_2),
                    (IID_BTN_3, &mut response_button_3),
                    (IID_BTN_4, &mut response_button_4),
                    (IID_BTN_5, &mut response_button_5),
                    (IID_BTN_6, &mut response_button_6),
                    (IID_BTN_7, &mut response_button_7),
                    (IID_BTN_8, &mut response_button_8),
                    (IID_BTN_9, &mut response_button_9),
                    (IID_EDITBOX, &mut response_editbox),
                ],
                ..Default::default()
            },
            |_ui, _bindings| {},
        );

        response
    });

    if first_show || response.response.clicked() {
        if let Some(response_editbox) = response_editbox {
            response_editbox.request_focus();
        }
    }

    if response_button_0.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('0');
    }

    if response_button_1.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('1');
    }

    if response_button_2.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('2');
    }

    if response_button_3.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('3');
    }

    if response_button_4.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('4');
    }

    if response_button_5.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('5');
    }

    if response_button_6.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('6');
    }

    if response_button_7.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('7');
    }

    if response_button_8.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('8');
    }

    if response_button_9.map_or(false, |x| x.clicked()) {
        active_dialog.current_value.push('9');
    }

    if response_button_del.map_or(false, |x| x.clicked()) && !active_dialog.current_value.is_empty()
    {
        active_dialog
            .current_value
            .remove(active_dialog.current_value.len() - 1);
    }

    if response_button_max.map_or(false, |x| x.clicked()) {
        if let Some(max_value) = active_dialog.max_value {
            active_dialog.current_value = format!("{}", max_value);
        }
    }

    if response_button_ok.map_or(false, |x| x.clicked()) {
        let active = ui_state.active.take().unwrap();
        let mut value = active.current_value.parse::<usize>().unwrap_or(0);

        if let Some(max_value) = active.max_value {
            if value > max_value {
                value = max_value;
            }
        }

        if value > 0 {
            if let Some(ok) = active.ok {
                ok(&mut commands, value);
            }
        } else if let Some(cancel) = active.cancel {
            cancel(&mut commands);
        }
    } else if response_button_cancel.map_or(false, |x| x.clicked()) {
        let active = ui_state.active.take().unwrap();

        if let Some(cancel) = active.cancel {
            cancel(&mut commands);
        }
    }
}
