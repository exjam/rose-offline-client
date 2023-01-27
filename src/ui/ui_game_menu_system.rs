use bevy::prelude::{Assets, Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};

use crate::{
    resources::UiResources,
    ui::{
        widgets::{DataBindings, Dialog},
        UiStateWindows,
    },
};

const IID_BTN_CHAR: i32 = 10;
const IID_BTN_ITEM: i32 = 11;
const IID_BTN_SKILL: i32 = 12;
const IID_BTN_QUEST: i32 = 13;
const IID_BTN_COMMUNITY: i32 = 14;
const IID_BTN_CLAN: i32 = 15;
const IID_BTN_HELP: i32 = 16;
const IID_BTN_INFO: i32 = 17;
const IID_BTN_OPTION: i32 = 18;
const IID_BTN_EXIT: i32 = 19;

#[derive(Default)]
pub struct UiGameMenuState {
    pub mouse_up_after_open: bool,
}

pub fn ui_game_menu_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_state_game_menu: Local<UiGameMenuState>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_game_menu) {
        dialog
    } else {
        return;
    };

    let mut response_button_character_info = None;
    let mut response_button_inventory = None;
    let mut response_button_skill_list = None;
    let mut response_button_quest_list = None;
    let mut response_button_options = None;
    let mut response_button_exit = None;
    let mut response_button_community = None;
    let mut response_button_clan = None;
    let mut response_button_help = None;
    let mut response_button_info = None;

    let response = egui::Window::new("Game Menu")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.menu_open)
        .title_bar(false)
        .resizable(false)
        .fixed_pos([dialog.adjust_x, dialog.adjust_y])
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    response: &mut [
                        (IID_BTN_CHAR, &mut response_button_character_info),
                        (IID_BTN_ITEM, &mut response_button_inventory),
                        (IID_BTN_SKILL, &mut response_button_skill_list),
                        (IID_BTN_QUEST, &mut response_button_quest_list),
                        (IID_BTN_COMMUNITY, &mut response_button_community),
                        (IID_BTN_CLAN, &mut response_button_clan),
                        (IID_BTN_HELP, &mut response_button_help),
                        (IID_BTN_INFO, &mut response_button_info),
                        (IID_BTN_OPTION, &mut response_button_options),
                        (IID_BTN_EXIT, &mut response_button_exit),
                    ],
                    ..Default::default()
                },
                |_, _| {},
            );
        });

    if let Some(response) = response {
        // To avoid clicked_elsewhere being triggered as soon as we open menu,
        // we will only look for it after we have detected all mouse buttons
        // have been released after opening
        if ui_state_game_menu.mouse_up_after_open {
            if response.response.clicked_elsewhere() {
                ui_state_windows.menu_open = false;
            }
        } else if !response.response.ctx.input().pointer.any_down() {
            ui_state_game_menu.mouse_up_after_open = true;
        }
    } else {
        ui_state_game_menu.mouse_up_after_open = false;
    }

    if response_button_character_info.map_or(false, |r| r.clicked()) {
        ui_state_windows.character_info_open = !ui_state_windows.character_info_open;
        ui_state_windows.menu_open = false;
    }

    if response_button_inventory.map_or(false, |r| r.clicked()) {
        ui_state_windows.inventory_open = !ui_state_windows.inventory_open;
        ui_state_windows.menu_open = false;
    }

    if response_button_skill_list.map_or(false, |r| r.clicked()) {
        ui_state_windows.skill_list_open = !ui_state_windows.skill_list_open;
        ui_state_windows.menu_open = false;
    }

    if response_button_quest_list.map_or(false, |r| r.clicked()) {
        ui_state_windows.quest_list_open = !ui_state_windows.quest_list_open;
        ui_state_windows.menu_open = false;
    }

    if response_button_options.map_or(false, |r| r.clicked()) {
        ui_state_windows.settings_open = !ui_state_windows.settings_open;
        ui_state_windows.menu_open = false;
    }

    if response_button_community.map_or(false, |r| r.clicked()) {
        // TODO: Community dialog
        ui_state_windows.menu_open = false;
    }

    if response_button_clan.map_or(false, |r| r.clicked()) {
        ui_state_windows.clan_open = !ui_state_windows.clan_open;
        ui_state_windows.menu_open = false;
    }

    if response_button_help.map_or(false, |r| r.clicked()) {
        // TODO: Help dialog
        ui_state_windows.menu_open = false;
    }

    if response_button_info.map_or(false, |r| r.clicked()) {
        // TODO: Info dialog
        ui_state_windows.menu_open = false;
    }

    if response_button_exit.map_or(false, |r| r.clicked()) {
        // TODO: Exit dialog
        ui_state_windows.menu_open = false;
    }

    if !egui_context.ctx_mut().wants_keyboard_input() {
        let mut input = egui_context.ctx_mut().input_mut();

        if input.consume_key(egui::Modifiers::ALT, egui::Key::A) {
            ui_state_windows.character_info_open = !ui_state_windows.character_info_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::I)
            || input.consume_key(egui::Modifiers::ALT, egui::Key::V)
        {
            ui_state_windows.inventory_open = !ui_state_windows.inventory_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::N) {
            ui_state_windows.clan_open = !ui_state_windows.clan_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::S) {
            ui_state_windows.skill_list_open = !ui_state_windows.skill_list_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::Q) {
            ui_state_windows.quest_list_open = !ui_state_windows.quest_list_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::O) {
            ui_state_windows.settings_open = !ui_state_windows.settings_open;
        }
    }
}
