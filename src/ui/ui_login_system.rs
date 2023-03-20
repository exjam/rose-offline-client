use bevy::{
    app::AppExit,
    prelude::{Assets, EventWriter, Local, Res},
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    events::LoginEvent,
    resources::{LoginState, ServerConfiguration, UiResources},
    ui::widgets::{DataBindings, Dialog},
};

const IID_EDIT_ID: i32 = 2;
const IID_EDIT_PWD: i32 = 3;
const IID_BTN_OK: i32 = 4;
const IID_BTN_CANCEL: i32 = 5;
const IID_CHECKBOX_SAVE_LASTCONECTID: i32 = 10;

#[derive(Default)]
pub struct UiStateLogin {
    username: String,
    password: String,
    remember_details: bool,
    initial_focus_set: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_login_system(
    mut ui_state: Local<UiStateLogin>,
    mut egui_context: EguiContexts,
    dialog_assets: Res<Assets<Dialog>>,
    login_state: Res<LoginState>,
    server_configuration: Res<ServerConfiguration>,
    ui_resources: Res<UiResources>,
    mut exit_events: EventWriter<AppExit>,
    mut login_events: EventWriter<LoginEvent>,
) {
    if !matches!(*login_state, LoginState::Input) {
        ui_state.initial_focus_set = false;
        return;
    }

    let mut ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_login) {
        dialog
    } else {
        return;
    };

    let mut response_username = None;
    let mut response_password = None;
    let mut response_ok = None;
    let mut response_cancel = None;
    let mut enter_pressed = false;

    let screen_size = egui_context
        .ctx_mut()
        .input(|input| input.screen_rect().size());
    let position = egui::pos2(screen_size.x - dialog.width - 100.0, 100.0);

    if !ui_state.initial_focus_set {
        if let Some(username) = server_configuration.preset_username.as_ref() {
            ui_state.username = username.clone();
        }

        if let Some(password) = server_configuration.preset_password.as_ref() {
            ui_state.password = password.clone();
        }
    }

    egui::Window::new("Login")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .fixed_pos(position)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    checked: &mut [(
                        IID_CHECKBOX_SAVE_LASTCONECTID,
                        &mut ui_state.remember_details,
                    )],
                    text: &mut [
                        (IID_EDIT_ID, &mut ui_state.username),
                        (IID_EDIT_PWD, &mut ui_state.password),
                    ],
                    response: &mut [
                        (IID_EDIT_ID, &mut response_username),
                        (IID_EDIT_PWD, &mut response_password),
                        (IID_BTN_OK, &mut response_ok),
                        (IID_BTN_CANCEL, &mut response_cancel),
                    ],
                    ..Default::default()
                },
                |ui, _| {
                    enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
                },
            )
        });

    if !ui_state.initial_focus_set {
        if let Some(r) = response_username.as_ref() {
            r.request_focus();
        }
        ui_state.initial_focus_set = true;
    }

    if enter_pressed || response_ok.map_or(false, |r| r.clicked()) {
        if ui_state.username.is_empty() {
            if let Some(r) = response_username.as_ref() {
                r.request_focus();
            }
        } else if ui_state.password.is_empty() {
            if let Some(r) = response_password.as_ref() {
                r.request_focus();
            }
        } else {
            login_events.send(LoginEvent::Login {
                username: ui_state.username.clone(),
                password: ui_state.password.clone(),
            });
        }
    }

    if response_cancel.map_or(false, |r| r.clicked()) {
        exit_events.send(AppExit);
    }
}
