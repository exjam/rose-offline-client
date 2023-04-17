use bevy::prelude::{Assets, Commands, EventWriter, Events, Local, Res, ResMut};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::egui::text::LayoutJob;

use crate::{
    events::MessageBoxEvent,
    resources::UiResources,
    ui::{
        widgets::{Dialog, DrawWidget, Widget},
        DataBindings, DialogInstance, UiSoundEvent,
    },
};

const IID_IMAGE_TOP: i32 = 5;
const IID_IMAGE_MIDDLE: i32 = 6;
const IID_IMAGE_BOTTOM: i32 = 7;
// const IID_LISTBOX_MESSAGE: i32 = 10;
const IID_BUTTON_OK: i32 = 255;
const IID_BUTTON_CANCEL: i32 = 256;

pub struct ActiveMessageBox {
    id: egui::Id,
    has_set_position: bool,
    dialog_instance: DialogInstance,
    message_layout_job: LayoutJob,
    modal: bool,
    ok: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
}

#[derive(Default)]
pub struct UiStateMessageBox {
    active: Vec<ActiveMessageBox>,
    window_ids: Vec<(bool, egui::Id)>,
}

pub fn ui_message_box_system(
    mut commands: Commands,
    mut ui_state: Local<UiStateMessageBox>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut egui_context: EguiContexts,
    mut message_box_events: ResMut<Events<MessageBoxEvent>>,
    dialog_assets: Res<Assets<Dialog>>,
    ui_resources: Res<UiResources>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_message_box) {
        dialog
    } else {
        return;
    };

    let image_top_height = if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_TOP) {
        image.height
    } else {
        26.0
    };

    let image_middle_height =
        if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_MIDDLE) {
            image.height
        } else {
            22.0
        };

    let image_bottom_height =
        if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_BOTTOM) {
            image.height
        } else {
            59.0
        };

    for event in message_box_events.drain() {
        let MessageBoxEvent::Show {
            message,
            modal,
            ok,
            cancel,
        } = event;

        let mut job = egui::text::LayoutJob::default();
        let current_text_format = egui::text::TextFormat {
            color: egui::Color32::WHITE,
            ..Default::default()
        };
        job.wrap.max_width = dialog.width - 16.0;
        job.append(&message, 0.0, current_text_format.clone());

        let id = if let Some((in_use, id)) =
            ui_state.window_ids.iter_mut().find(|(in_use, _)| !in_use)
        {
            *in_use = true;
            *id
        } else {
            let id = egui::Id::new(format!("msgbox_{}", ui_state.window_ids.len()));
            ui_state.window_ids.push((true, id));
            id
        };

        ui_state.active.push(ActiveMessageBox {
            id,
            dialog_instance: DialogInstance::new("MSGBOX.XML"),
            has_set_position: false,
            message_layout_job: job,
            modal,
            ok,
            cancel,
        });
    }

    if ui_state.active.iter().any(|x| x.modal) {
        egui::Area::new("modal_msgbox")
            .interactable(true)
            .fixed_pos(egui::Pos2::ZERO)
            .show(egui_context.ctx_mut(), |ui| {
                let interceptor_rect = ui.ctx().input(|input| input.screen_rect());

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

    let mut i = 0;
    while i < ui_state.active.len() {
        let active_message_box = &mut ui_state.active[i];
        let dialog = if let Some(dialog) = active_message_box
            .dialog_instance
            .get_mut(&dialog_assets, &ui_resources)
        {
            dialog
        } else {
            i += 1;
            continue;
        };

        let (message_galley, num_image_middle) = egui_context.ctx_mut().fonts(|fonts| {
            let message_galley = fonts.layout_job(active_message_box.message_layout_job.clone());
            let message_size = message_galley.size();
            let num_image_middle = 1 + (message_size.y / image_middle_height) as usize;
            (message_galley, num_image_middle)
        });

        let dialog_width = dialog.width;
        let dialog_height =
            image_top_height + image_middle_height * num_image_middle as f32 + image_bottom_height;

        let screen_size = egui_context
            .ctx_mut()
            .input(|input| input.screen_rect().size());
        let default_x = (screen_size.x / 2.0 - dialog.width / 2.0) + (i * 20) as f32;
        let default_y = (screen_size.y / 2.0 - dialog_height / 2.0) + (i * 20) as f32;

        if let Some(Widget::Image(image)) = dialog.get_widget_mut(IID_IMAGE_MIDDLE) {
            image.y = image_top_height;
        }

        if let Some(Widget::Image(image)) = dialog.get_widget_mut(IID_IMAGE_BOTTOM) {
            image.y = image_top_height + image_middle_height * num_image_middle as f32;
        }

        let has_cancel_button = active_message_box.cancel.is_some();
        let has_ok_button = !has_cancel_button || active_message_box.ok.is_some();

        if has_ok_button {
            if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BUTTON_OK) {
                if has_cancel_button {
                    button.x = dialog_width / 4.0 - button.width / 2.0;
                } else {
                    button.x = dialog_width / 2.0 - button.width / 2.0;
                }

                button.y = image_top_height
                    + image_middle_height * num_image_middle as f32
                    + image_bottom_height / 2.0
                    - button.height / 2.0;
            }
        }

        if has_cancel_button {
            if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BUTTON_CANCEL) {
                if has_ok_button {
                    button.x = dialog_width * 3.0 / 4.0 - button.width / 2.0;
                } else {
                    button.x = dialog_width / 2.0 - button.width / 2.0;
                }

                button.y = image_top_height
                    + image_middle_height * num_image_middle as f32
                    + image_bottom_height / 2.0
                    - button.height / 2.0;
            }
        }

        let mut response_button_ok = None;
        let mut response_button_cancel = None;

        let mut area = egui::Area::new(active_message_box.id)
            .movable(true)
            .interactable(true)
            .default_pos([default_x, default_y])
            .order(egui::Order::Foreground);

        if !active_message_box.has_set_position {
            area = area.current_pos([default_x, default_y]);
            active_message_box.has_set_position = true;
        }

        area.show(egui_context.ctx_mut(), |ui| {
            let response = ui.allocate_response(
                egui::vec2(dialog_width, dialog_height),
                egui::Sense::hover(),
            );

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    visible: &mut [(IID_BUTTON_OK, false), (IID_BUTTON_CANCEL, false)],
                    response: &mut [
                        (IID_BUTTON_OK, &mut response_button_ok),
                        (IID_BUTTON_CANCEL, &mut response_button_cancel),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    if let Some(Widget::Image(image)) = dialog.get_widget(IID_IMAGE_MIDDLE) {
                        if let Some(sprite) = image.sprite.as_ref() {
                            let mut pos = ui.min_rect().min;
                            pos.y += image_top_height;

                            for _ in 1..num_image_middle {
                                pos.y += image_middle_height;
                                sprite.draw(ui, pos);
                            }
                        }
                    }

                    let message_rect = egui::Rect::from_min_size(
                        ui.min_rect().min + egui::vec2(0.0, image_top_height),
                        egui::vec2(dialog_width, image_middle_height * num_image_middle as f32),
                    );
                    ui.allocate_ui_at_rect(message_rect, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.add(egui::Label::new(message_galley.clone()))
                        })
                    });

                    bindings.visible = &mut [];
                    if has_ok_button {
                        if let Some(Widget::Button(button)) = dialog.get_widget(IID_BUTTON_OK) {
                            button.draw_widget(ui, bindings);
                        }
                    }

                    if has_cancel_button {
                        if let Some(Widget::Button(button)) = dialog.get_widget(IID_BUTTON_CANCEL) {
                            button.draw_widget(ui, bindings);
                        }
                    }
                },
            );

            response
        });

        if response_button_ok.map_or(false, |x| x.clicked()) {
            let active_message_box = ui_state.active.remove(i);

            if let Some(ok) = active_message_box.ok {
                ok(&mut commands);
            }

            if let Some((in_use, _)) = ui_state
                .window_ids
                .iter_mut()
                .find(|(_, id)| *id == active_message_box.id)
            {
                *in_use = false;
            }

            continue;
        }

        if response_button_cancel.map_or(false, |x| x.clicked()) {
            let active_message_box = ui_state.active.remove(i);

            if let Some(cancel) = active_message_box.cancel {
                cancel(&mut commands);
            }

            if let Some((in_use, _)) = ui_state
                .window_ids
                .iter_mut()
                .find(|(_, id)| *id == active_message_box.id)
            {
                *in_use = false;
            }

            continue;
        }

        i += 1;
    }
}
