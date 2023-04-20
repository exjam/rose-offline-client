use bevy::prelude::{Assets, EventWriter, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::messages::{client::ClientMessage, PartyItemSharing, PartyXpSharing};

use crate::{
    components::{PartyInfo, PartyOwner, PlayerCharacter},
    resources::{GameConnection, UiResources},
    ui::{
        widgets::{DataBindings, Dialog, DrawText},
        UiSoundEvent, UiStateWindows,
    },
};

use super::widgets::Widget;

const IID_BTN_CLOSE: i32 = 10;
const IID_BTN_CONFIRM: i32 = 11;
const IID_RADIOBOX_ITEM: i32 = 34;
const IID_RADIOBUTTON_ITEM_PICK: i32 = 35;
const IID_RADIOBUTTON_ITEM_SEQUENCE: i32 = 36;
const IID_RADIOBOX_EXP: i32 = 37;
const IID_RADIOBUTTON_EXP_EQUALITY: i32 = 38;
const IID_RADIOBUTTON_EXP_RATIO_LEVEL: i32 = 39;
const IID_CHECKBOX_SHOW_PARTYMEMBER_HPGUAGE: i32 = 40;

pub struct UiStatePartyOptionSystem {
    item_sharing_rule: i32,
    exp_sharing_rule: i32,
    show_party_member_hp_gauge: bool,
}

impl Default for UiStatePartyOptionSystem {
    fn default() -> Self {
        Self {
            item_sharing_rule: 0,
            exp_sharing_rule: 0,
            show_party_member_hp_gauge: true,
        }
    }
}

pub fn ui_party_option_system(
    mut ui_state: Local<UiStatePartyOptionSystem>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut egui_context: EguiContexts,
    mut query_party_info: Query<&PartyInfo, With<PlayerCharacter>>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    game_connection: Option<Res<GameConnection>>,
) {
    let ui_state = &mut *ui_state;
    let party_dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_party) {
        dialog
    } else {
        return;
    };

    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_party_option) {
        dialog
    } else {
        return;
    };

    let party_info = if let Ok(party_info) = query_party_info.get_single_mut() {
        party_info
    } else {
        return;
    };

    if !ui_state_windows.party_options_open {
        ui_state.item_sharing_rule = match party_info.item_sharing {
            PartyItemSharing::EqualLootDistribution => IID_RADIOBUTTON_ITEM_PICK,
            PartyItemSharing::AcquisitionOrder => IID_RADIOBUTTON_ITEM_SEQUENCE,
        };

        ui_state.exp_sharing_rule = match party_info.xp_sharing {
            PartyXpSharing::EqualShare => IID_RADIOBUTTON_EXP_EQUALITY,
            PartyXpSharing::DistributedByLevel => IID_RADIOBUTTON_EXP_RATIO_LEVEL,
        };
        return;
    }

    let mut response_close_button = None;
    let mut response_confirm_button = None;
    let player_is_owner = matches!(party_info.owner, PartyOwner::Player);

    egui::Window::new("Party Options")
        .anchor(egui::Align2::RIGHT_CENTER, [-party_dialog.width, 0.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    checked: &mut [(
                        IID_CHECKBOX_SHOW_PARTYMEMBER_HPGUAGE,
                        &mut ui_state.show_party_member_hp_gauge,
                    )],
                    enabled: &mut [
                        (IID_RADIOBUTTON_ITEM_PICK, player_is_owner),
                        (IID_RADIOBUTTON_ITEM_SEQUENCE, player_is_owner),
                        (IID_RADIOBUTTON_EXP_EQUALITY, player_is_owner),
                        (IID_RADIOBUTTON_EXP_RATIO_LEVEL, player_is_owner),
                        (IID_CHECKBOX_SHOW_PARTYMEMBER_HPGUAGE, player_is_owner),
                    ],
                    radio: &mut [
                        (IID_RADIOBOX_ITEM, &mut ui_state.item_sharing_rule),
                        (IID_RADIOBOX_EXP, &mut ui_state.exp_sharing_rule),
                    ],
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_CONFIRM, &mut response_confirm_button),
                    ],
                    visible: &mut [(IID_BTN_CONFIRM, player_is_owner)],
                    ..Default::default()
                },
                |ui, _bindings| {
                    ui.add_label_at(
                        egui::pos2(20.0, 38.0),
                        egui::RichText::new("EXP Distribution:").color(egui::Color32::BLACK),
                    );
                    ui.add_label_at(
                        egui::pos2(38.0, 55.0),
                        egui::RichText::new("Equal Share").color(egui::Color32::BLACK),
                    );
                    ui.add_label_at(
                        egui::pos2(38.0, 77.0),
                        egui::RichText::new("Distributed by Level").color(egui::Color32::BLACK),
                    );

                    ui.add_label_at(
                        egui::pos2(20.0, 104.0),
                        egui::RichText::new("Item Earning Priority:").color(egui::Color32::BLACK),
                    );
                    ui.add_label_at(
                        egui::pos2(38.0, 122.0),
                        egui::RichText::new("Equal Loot Distribution").color(egui::Color32::BLACK),
                    );
                    ui.add_label_at(
                        egui::pos2(38.0, 144.0),
                        egui::RichText::new("Acquisition Order").color(egui::Color32::BLACK),
                    );

                    ui.add_label_at(
                        egui::pos2(38.0, 182.0),
                        egui::RichText::new("HP Gauge (Members)").color(egui::Color32::BLACK),
                    );

                    if let Some(Widget::Button(button)) = dialog.get_widget(IID_BTN_CONFIRM) {
                        ui.put(
                            button.widget_rect(ui.min_rect().min),
                            egui::Label::new(
                                egui::RichText::new("Confirm").color(egui::Color32::BLACK),
                            ),
                        );
                    }
                },
            );
        });

    if response_close_button.map_or(false, |x| x.clicked()) {
        ui_state_windows.party_options_open = false;
    }

    if response_confirm_button.map_or(false, |x| x.clicked()) {
        if let Some(game_connection) = &game_connection {
            game_connection
                .client_message_tx
                .send(ClientMessage::PartyUpdateRules {
                    item_sharing: if ui_state.item_sharing_rule == IID_RADIOBUTTON_ITEM_PICK {
                        PartyItemSharing::EqualLootDistribution
                    } else {
                        PartyItemSharing::AcquisitionOrder
                    },
                    xp_sharing: if ui_state.exp_sharing_rule == IID_RADIOBUTTON_EXP_EQUALITY {
                        PartyXpSharing::EqualShare
                    } else {
                        PartyXpSharing::DistributedByLevel
                    },
                })
                .ok();
        }

        ui_state_windows.party_options_open = false;
    }
}
