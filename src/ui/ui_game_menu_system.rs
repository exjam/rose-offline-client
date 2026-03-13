use bevy::prelude::{Assets, EventWriter, Res, ResMut};
use bevy_egui::{egui, EguiContexts};

use crate::{
    resources::UiResources,
    ui::{
        widgets::{DataBindings, Dialog},
        UiSoundEvent, UiStateWindows,
    },
    Config,
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

pub fn ui_game_menu_system(
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    ui_resources: Res<UiResources>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    dialog_assets: Res<Assets<Dialog>>,
    config: Res<Config>,
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

    egui::Window::new("Game Menu")
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
                    sound_events: Some(&mut ui_sound_events),
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

    if response_button_character_info.map_or(false, |r| r.clicked()) {
        ui_state_windows.character_info_open = !ui_state_windows.character_info_open;
    }

    if response_button_inventory.map_or(false, |r| r.clicked()) {
        ui_state_windows.inventory_open = !ui_state_windows.inventory_open;
    }

    if response_button_skill_list.map_or(false, |r| r.clicked()) {
        ui_state_windows.skill_list_open = !ui_state_windows.skill_list_open;
    }

    if response_button_quest_list.map_or(false, |r| r.clicked()) {
        ui_state_windows.quest_list_open = !ui_state_windows.quest_list_open;
    }

    if response_button_options.map_or(false, |r| r.clicked()) {
        ui_state_windows.settings_open = !ui_state_windows.settings_open;
    }

    if response_button_community.map_or(false, |r| r.clicked()) {
        // TODO: Community dialog
    }

    if response_button_clan.map_or(false, |r| r.clicked()) {
        ui_state_windows.clan_open = !ui_state_windows.clan_open;
    }

    if response_button_help.map_or(false, |r| r.clicked()) {
        // TODO: Help dialog
    }

    if response_button_info.map_or(false, |r| r.clicked()) {
        // TODO: Info dialog
    }

    if response_button_exit.map_or(false, |r| r.clicked()) {
        // TODO: Exit dialog
    }

    if !egui_context.ctx_mut().wants_keyboard_input() {
        egui_context.ctx_mut().input_mut(|input| {
            if input.consume_shortcut(&config.hotkeys.character) {
                ui_state_windows.character_info_open = !ui_state_windows.character_info_open;
            }

            if input.consume_shortcut(&config.hotkeys.inventory) {
                ui_state_windows.inventory_open = !ui_state_windows.inventory_open;
            }

            if input.consume_shortcut(&config.hotkeys.clan) {
                ui_state_windows.clan_open = !ui_state_windows.clan_open;
            }

            if input.consume_shortcut(&config.hotkeys.skills) {
                ui_state_windows.skill_list_open = !ui_state_windows.skill_list_open;
            }

            if input.consume_shortcut(&config.hotkeys.quests) {
                ui_state_windows.quest_list_open = !ui_state_windows.quest_list_open;
            }

            if input.consume_shortcut(&config.hotkeys.settings) {
                ui_state_windows.settings_open = !ui_state_windows.settings_open;
            }
        });
    }
}
