use bevy::prelude::{
    AssetServer, Assets, Camera3d, Commands, Entity, EventWriter, Local, Query, Res, ResMut, With,
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    animation::CameraAnimation,
    events::CharacterSelectEvent,
    resources::{CharacterList, CharacterSelectState, GameData, UiResources},
    ui::{
        widgets::{DataBindings, Dialog, Widget},
        DialogInstance,
    },
};

pub struct UiCharacterSelectState {
    dialog_instance: DialogInstance,
}

impl Default for UiCharacterSelectState {
    fn default() -> Self {
        Self {
            dialog_instance: DialogInstance::new("DLGSELAVATAR.XML"),
        }
    }
}

const IID_BTN_CREATE: i32 = 10;
const IID_BTN_DELETE: i32 = 11;
const IID_BTN_OK: i32 = 12;
const IID_BTN_CANCEL: i32 = 13;

pub fn ui_character_select_system(
    mut commands: Commands,
    mut ui_state: Local<UiCharacterSelectState>,
    mut character_select_state: ResMut<CharacterSelectState>,
    mut egui_context: EguiContexts,
    query_camera: Query<Entity, With<Camera3d>>,
    character_list: Option<Res<CharacterList>>,
    asset_server: Res<AssetServer>,
    dialog_assets: Res<Assets<Dialog>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    mut character_select_events: EventWriter<CharacterSelectEvent>,
) {
    let ui_state = &mut *ui_state;
    if !matches!(
        *character_select_state,
        CharacterSelectState::CharacterSelect(_)
    ) {
        return;
    }

    let dialog = if let Some(dialog) = ui_state
        .dialog_instance
        .get_mut(&dialog_assets, &ui_resources)
    {
        dialog
    } else {
        return;
    };

    let screen_size = egui_context
        .ctx_mut()
        .input(|input| input.screen_rect().size());

    if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_CANCEL) {
        button.x = screen_size.x / 5.0 - button.width / 2.0;
    }

    if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_CREATE) {
        button.x = screen_size.x * 2.0 / 5.0 - button.width / 2.0;
    }

    if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_DELETE) {
        button.x = screen_size.x * 3.0 / 5.0 - button.width / 2.0;
    }

    if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_OK) {
        button.x = screen_size.x * 4.0 / 5.0 - button.width / 2.0;
    }

    let mut response_create_button = None;
    let mut response_delete_button = None;
    let mut response_ok_button = None;
    let mut response_cancel_button = None;

    egui::Window::new("Character Select")
        .anchor(egui::Align2::LEFT_BOTTOM, [0.0, -24.0 - 40.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(screen_size.x)
        .default_height(40.0)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    response: &mut [
                        (IID_BTN_CREATE, &mut response_create_button),
                        (IID_BTN_DELETE, &mut response_delete_button),
                        (IID_BTN_OK, &mut response_ok_button),
                        (IID_BTN_CANCEL, &mut response_cancel_button),
                    ],
                    ..Default::default()
                },
                |_, _| {},
            );
        });

    if response_create_button.map_or(false, |r| r.clicked())
        && character_list.as_ref().map_or(true, |character_list| {
            character_list.characters.len() < game_data.character_select_positions.len()
        })
    {
        commands
            .entity(query_camera.single())
            .insert(CameraAnimation::once(
                asset_server.load("3DDATA/TITLE/CAMERA01_CREATE01.ZMO"),
            ));

        *character_select_state = CharacterSelectState::CharacterCreate;
    }

    if response_delete_button.map_or(false, |r| r.clicked()) {
        character_select_events.send(CharacterSelectEvent::DeleteSelected);
    }

    if response_ok_button.map_or(false, |r| r.clicked()) {
        character_select_events.send(CharacterSelectEvent::PlaySelected);
    }

    if response_cancel_button.map_or(false, |r| r.clicked()) {
        character_select_events.send(CharacterSelectEvent::Disconnect);
    }
}
