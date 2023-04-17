use bevy::prelude::{Assets, EventWriter, Handle, Local, Res};

use crate::{
    resources::UiResources,
    ui::{widgets::Dialog, UiSoundEvent, UiStateWindows},
};

pub fn ui_window_sound_system(
    mut state: Local<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    next: Res<UiStateWindows>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let mut play_dialog_sound =
        |state: &mut bool, next_state: bool, dialog_asset: &Handle<Dialog>| {
            if *state != next_state {
                if let Some(dialog) = dialog_assets.get(dialog_asset) {
                    let sound_id = if next_state {
                        dialog.show_sound_id
                    } else {
                        dialog.hide_sound_id
                    };

                    if let Some(sound_id) = sound_id {
                        ui_sound_events.send(UiSoundEvent::new(sound_id));
                    }
                }

                *state = next_state;
            }
        };

    play_dialog_sound(
        &mut state.character_info_open,
        next.character_info_open,
        &ui_resources.dialog_character_info,
    );
    play_dialog_sound(
        &mut state.clan_open,
        next.clan_open,
        &ui_resources.dialog_clan,
    );
    play_dialog_sound(
        &mut state.inventory_open,
        next.inventory_open,
        &ui_resources.dialog_inventory,
    );
    play_dialog_sound(
        &mut state.skill_list_open,
        next.skill_list_open,
        &ui_resources.dialog_skill_list,
    );
    play_dialog_sound(
        &mut state.skill_tree_open,
        next.skill_tree_open,
        &ui_resources.dialog_skill_tree,
    );
    play_dialog_sound(
        &mut state.quest_list_open,
        next.quest_list_open,
        &ui_resources.dialog_quest_list,
    );
    // play_dialog_sound(&mut state.settings_open, next.settings_open, &ui_resources.dialog_..);
    play_dialog_sound(
        &mut state.menu_open,
        next.menu_open,
        &ui_resources.dialog_game_menu,
    );
    play_dialog_sound(
        &mut state.party_open,
        next.party_open,
        &ui_resources.dialog_party,
    );
    play_dialog_sound(
        &mut state.party_options_open,
        next.party_options_open,
        &ui_resources.dialog_party_option,
    );

    play_dialog_sound(
        &mut state.bank_open,
        next.bank_open,
        &ui_resources.dialog_bank,
    );
    play_dialog_sound(
        &mut state.create_clan_open,
        next.create_clan_open,
        &ui_resources.dialog_create_clan,
    );
}
