use bevy::prelude::{Assets, EventWriter, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContexts};
use rose_data::ClanMemberPosition;

use crate::{
    components::{Clan, ClanMembership, PlayerCharacter},
    resources::{GameData, UiResources},
    ui::{
        widgets::{DataBindings, Dialog, DrawText},
        UiSoundEvent, UiStateWindows,
    },
};

use super::widgets::Widget;

const IID_BTN_CLOSE: i32 = 10;
const IID_TABBEDPANE: i32 = 20;
const IID_TAB_INFO: i32 = 21;
const IID_TAB_MEMBER: i32 = 31;
const IID_TAB_SKILL: i32 = 51;
const IID_TAB_NOTICE: i32 = 61;
const IID_ZLIST_MEMBER: i32 = 36;
/*
const IID_BTN_ICONIZE: i32 = 11;
const IID_BTN_TAB_INFO: i32 = 25;
const IID_BTN_PREVIEW: i32 = 28;
const IID_BTN_REGIST_CLANMARK: i32 = 29;
const IID_BTN_TAB_MEMBER: i32 = 35;
const IID_BTN_ENTRUST: i32 = 41;
const IID_BTN_BAN: i32 = 42;
const IID_BTN_CLASS_UP: i32 = 43;
const IID_BTN_CLASS_DOWN: i32 = 44;
const IID_BTN_REQJOIN: i32 = 45;
const IID_BTN_WITHDRAWAL: i32 = 46;
const IID_BTN_TAB_SKILL: i32 = 55;
const IID_ZLIST_SKILL: i32 = 56;
const IID_BTN_TAB_NOTICE: i32 = 70;
const IID_ZLIST_NOTICE: i32 = 75;
const IID_ZLIST_NOTICE_CONTENT: i32 = 77;
const IID_BTN_REGIST_NOTICE: i32 = 80;
const IID_BTN_DELETE_NOTICE: i32 = 81;
*/

pub struct UiStateClan {
    current_tab: i32,
    scroll_index_members: i32,
    selected_member_index: i32,
}

impl Default for UiStateClan {
    fn default() -> Self {
        Self {
            current_tab: IID_TAB_INFO,
            scroll_index_members: 0,
            selected_member_index: 0,
        }
    }
}

pub fn ui_clan_system(
    mut egui_context: EguiContexts,
    query_clan: Query<(&Clan, &ClanMembership), With<PlayerCharacter>>,
    mut ui_state: Local<UiStateClan>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    game_data: Res<GameData>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_clan) {
        dialog
    } else {
        return;
    };

    let ui_state = &mut *ui_state;
    let mut response_close_button = None;

    egui::Window::new("Clan")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.clan_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            let Ok((clan, clan_membership)) = query_clan.get_single() else {
                return
            };

            let member_listbox_extent =
                if let Some(Widget::ZListbox(listbox)) = dialog.get_widget(IID_ZLIST_MEMBER) {
                    listbox.extent
                } else {
                    1
                };
            let member_scrollbar_range = 0..clan.members.len() as i32;

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [(IID_BTN_CLOSE, &mut response_close_button)],
                    tabs: &mut [(IID_TABBEDPANE, &mut ui_state.current_tab)],
                    scroll: &mut [(
                        IID_ZLIST_MEMBER,
                        (
                            &mut ui_state.scroll_index_members,
                            member_scrollbar_range,
                            member_listbox_extent,
                        ),
                    )],
                    zlist: &mut [(
                        IID_ZLIST_MEMBER,
                        (
                            &mut ui_state.selected_member_index,
                            &|ui, index, is_selected| {
                                let (_rect, response) = ui.allocate_exact_size(
                                    egui::vec2(200.0, 18.0),
                                    egui::Sense::click(),
                                );

                                if let Some(member) = clan.members.get(index as usize) {
                                    if matches!(member.position, ClanMemberPosition::Penalty) {
                                        egui::Color32::RED
                                    } else if is_selected {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::BLACK
                                    };

                                    ui.add_label_at(
                                        egui::pos2(2.0, 2.0),
                                        egui::RichText::new(format!(
                                            "{} ({})",
                                            member.name,
                                            game_data
                                                .string_database
                                                .get_clan_member_position(member.position)
                                        ))
                                        .color(egui::Color32::BLACK),
                                    );
                                }

                                response
                            },
                        ),
                    )],
                    ..Default::default()
                },
                |ui, bindings| match bindings.get_tab(IID_TABBEDPANE) {
                    Some(&mut IID_TAB_INFO) => {
                        ui.add_label_at(
                            egui::pos2(15.0, 73.0),
                            egui::RichText::new(game_data.client_strings.clan_name)
                                .color(egui::Color32::BLACK),
                        );
                        ui.add_label_at(egui::pos2(88.0, 73.0), &clan.name);

                        ui.add_label_at(
                            egui::pos2(15.0, 94.0),
                            egui::RichText::new(game_data.client_strings.clan_level)
                                .color(egui::Color32::BLACK),
                        );
                        ui.add_label_at(egui::pos2(88.0, 94.0), format!("{}", clan.level.0));

                        ui.add_label_at(
                            egui::pos2(15.0, 115.0),
                            egui::RichText::new(game_data.client_strings.clan_point)
                                .color(egui::Color32::BLACK),
                        );
                        ui.add_label_at(egui::pos2(88.0, 115.0), format!("{}", clan.points.0));

                        ui.add_label_at(
                            egui::pos2(15.0, 135.0),
                            egui::RichText::new(game_data.client_strings.clan_slogan)
                                .color(egui::Color32::BLACK),
                        );
                        ui.add_label_in(
                            egui::Rect::from_min_max(
                                egui::pos2(88.0, 135.0),
                                egui::pos2(210.0, 180.0),
                            ),
                            &clan.description,
                        );

                        ui.add_label_at(
                            egui::pos2(15.0, 187.0),
                            egui::RichText::new(game_data.client_strings.clan_money)
                                .color(egui::Color32::BLACK),
                        );
                        ui.add_label_at(egui::pos2(88.0, 187.0), format!("{}", clan.money.0));

                        let max_members = game_data
                            .ability_value_calculator
                            .calculate_clan_max_members(clan.level.0);
                        ui.add_label_at(
                            egui::pos2(15.0, 208.0),
                            egui::RichText::new(game_data.client_strings.clan_member_count)
                                .color(egui::Color32::BLACK),
                        );
                        ui.add_label_at(
                            egui::pos2(88.0, 208.0),
                            format!("{} / {}", clan.members.len(), max_members),
                        );

                        // TODO: Clan Mark register time
                        // ui.add_label_at(egui::pos2(15.0, 229.0),  game_data.client_strings.clan_mark_register_time);
                        // ui.add_label_at(egui::pos2(88.0, 229.0),  format!("{}", clan.clan_mark_register_time));

                        ui.add_label_at(
                            egui::pos2(15.0, 248.0),
                            egui::RichText::new(game_data.client_strings.clan_ally)
                                .color(egui::Color32::BLACK),
                        );

                        ui.add_label_at(
                            egui::pos2(84.0, 288.0),
                            game_data
                                .string_database
                                .get_clan_member_position(clan_membership.position),
                        );
                        ui.add_label_at(
                            egui::pos2(88.0, 309.0),
                            format!("{}", clan_membership.contribution.0),
                        );
                    }
                    Some(&mut IID_TAB_MEMBER) => {}
                    Some(&mut IID_TAB_SKILL) => {}
                    Some(&mut IID_TAB_NOTICE) => {
                        ui.add_label_in(
                            egui::Rect::from_min_max(
                                egui::pos2(30.0, 75.0),
                                egui::pos2(190.0, 310.0),
                            ),
                            "TODO: Notice",
                        );
                    }
                    _ => {}
                },
            );
        });

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.clan_open = false;
    }
}
