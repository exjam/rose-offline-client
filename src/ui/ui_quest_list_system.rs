use bevy::prelude::{Assets, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_data::Item;
use rose_game_common::components::QuestState;

use crate::{
    components::PlayerCharacter,
    resources::{GameData, UiResources},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem},
        ui_add_item_tooltip,
        widgets::{DataBindings, Dialog, DrawText, Widget},
        DragAndDropId, DragAndDropSlot, UiStateWindows,
    },
};

use super::DialogInstance;

// const IID_BTN_ABANDON: i32 = 50;
const IID_BTN_CLOSE: i32 = 10;
// const IID_BTN_ICONIZE: i32 = 11;
const IID_BTN_MINIMIZE: i32 = 113;
const IID_BTN_MAXIMIZE: i32 = 114;
const IID_ZLIST_QUEST: i32 = 20;
const IID_ZLIST_SCROLLBAR: i32 = 21;
const IID_LIST_QUESTINFO: i32 = 30;
// const IID_ZLIST_ITEM: i32 = 99;
// const IID_PANE_QUESTLIST: i32 = 100;
const IID_PANE_QUESTINFO: i32 = 200;

fn ui_add_quest_item_slot(
    ui: &mut egui::Ui,
    pos: egui::Pos2,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    item: Option<&Item>,
    game_data: &GameData,
    ui_resources: &UiResources,
) {
    let mut dragged_item = None;
    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(pos, egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::with_item(
                        DragAndDropId::NotDraggable,
                        item,
                        None,
                        game_data,
                        ui_resources,
                        |_| false,
                        &mut dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if let Some(item) = item {
        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, player_tooltip_data, item);
        });
    }
}

pub struct UiQuestListState {
    pub dialog_instance: DialogInstance,
    pub scroll_index: i32,
    pub selected_index: i32,
    pub minimised: bool,
}

impl Default for UiQuestListState {
    fn default() -> Self {
        Self {
            dialog_instance: DialogInstance::new("DLGQUEST.XML"),
            scroll_index: 0,
            selected_index: 0,
            minimised: false,
        }
    }
}

pub fn ui_quest_list_system(
    mut ui_state: Local<UiQuestListState>,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<&QuestState, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = ui_state
        .dialog_instance
        .get_mut(&dialog_assets, &ui_resources)
    {
        dialog
    } else {
        return;
    };
    let player_quest_state = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    let listbox_extent = if let Some(Widget::ZListbox(listbox)) = dialog.get_widget(IID_ZLIST_QUEST)
    {
        listbox.extent
    } else {
        1
    };
    let num_quests = player_quest_state
        .active_quests
        .iter()
        .filter(|q| q.is_some())
        .count();
    let scrollbar_range = 0..num_quests as i32;

    let mut response_close_button = None;
    let mut response_minimise_button = None;
    let mut response_maximise_button = None;
    let is_minimised = ui_state.minimised;

    egui::Window::new("Quest List")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.quest_list_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    visible: &mut [
                        (IID_ZLIST_SCROLLBAR, !is_minimised),
                        (IID_ZLIST_QUEST, !is_minimised),
                        (IID_BTN_MINIMIZE, !is_minimised),
                        (IID_BTN_MAXIMIZE, is_minimised),
                    ],
                    scroll: &mut [(
                        IID_ZLIST_QUEST,
                        (&mut ui_state.scroll_index, scrollbar_range, listbox_extent),
                    )],
                    zlist: &mut [(
                        IID_ZLIST_QUEST,
                        (&mut ui_state.selected_index, &|ui, index, selected| {
                            let (_rect, response) = ui
                                .allocate_exact_size(egui::vec2(174.0, 24.0), egui::Sense::click());

                            if let Some(active_quest) = player_quest_state
                                .active_quests
                                .iter()
                                .filter(|q| q.is_some())
                                .nth(index as usize)
                                .and_then(|x| x.as_ref())
                            {
                                if let Some(quest_data) =
                                    game_data.quests.get_quest_data(active_quest.quest_id)
                                {
                                    if selected {
                                        ui.add_label_at(
                                            egui::pos2(28.0, 4.0),
                                            egui::RichText::new(quest_data.name)
                                                .color(egui::Color32::YELLOW),
                                        );
                                    } else {
                                        ui.add_label_at(egui::pos2(28.0, 4.0), quest_data.name);
                                    }
                                }
                            }

                            response
                        }),
                    )],
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_MINIMIZE, &mut response_minimise_button),
                        (IID_BTN_MAXIMIZE, &mut response_maximise_button),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    let selected_quest_index = bindings
                        .get_zlist_selected_index(IID_ZLIST_QUEST)
                        .unwrap_or(0);

                    if let Some(selected_quest) = player_quest_state
                        .active_quests
                        .iter()
                        .filter(|q| q.is_some())
                        .nth(selected_quest_index as usize)
                        .and_then(|x| x.as_ref())
                    {
                        let quest_data = game_data.quests.get_quest_data(selected_quest.quest_id);

                        let rect_info = if let Some(Widget::Pane(pane)) =
                            dialog.get_widget(IID_PANE_QUESTINFO)
                        {
                            pane.widget_rect(ui.min_rect().min)
                        } else {
                            ui.min_rect()
                        };

                        if let Some(quest_data) = quest_data {
                            ui.allocate_ui_at_rect(
                                rect_info.translate(egui::vec2(43.0, 38.0)),
                                |ui| {
                                    ui.horizontal_top(|ui| {
                                        ui.add(egui::Label::new(
                                            egui::RichText::new(quest_data.name)
                                                .color(egui::Color32::YELLOW),
                                        ));
                                    })
                                },
                            );

                            // TODO: Add quest icon

                            if let Some(Widget::Listbox(listbox)) =
                                dialog.get_widget(IID_LIST_QUESTINFO)
                            {
                                let rect = listbox.widget_rect(rect_info.min);

                                ui.allocate_ui_at_rect(rect, |ui| {
                                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(
                                        ui,
                                        |ui| {
                                            ui.label(quest_data.description);
                                        },
                                    );
                                });
                            }
                        }

                        const QUEST_ITEM_SLOT_POS: [egui::Vec2; 6] = [
                            egui::vec2(10.0, 176.0),
                            egui::vec2(51.0, 176.0),
                            egui::vec2(92.0, 176.0),
                            egui::vec2(133.0, 176.0),
                            egui::vec2(174.0, 176.0),
                            egui::vec2(211.0, 176.0),
                        ];

                        for (i, item) in selected_quest.items.iter().enumerate() {
                            ui_add_quest_item_slot(
                                ui,
                                rect_info.min + QUEST_ITEM_SLOT_POS[i],
                                player_tooltip_data.as_ref(),
                                item.as_ref(),
                                &game_data,
                                &ui_resources,
                            );
                        }
                    }
                },
            );
        });

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.quest_list_open = false;
    }

    if response_minimise_button.map_or(false, |r| r.clicked()) {
        ui_state.minimised = true;

        if let Some(Widget::Pane(pane)) = dialog.get_widget_mut(IID_PANE_QUESTINFO) {
            pane.y = 56.0;
        }
    }

    if response_maximise_button.map_or(false, |r| r.clicked()) {
        ui_state.minimised = false;

        if let Some(Widget::Pane(pane)) = dialog.get_widget_mut(IID_PANE_QUESTINFO) {
            pane.y = 171.0;
        }
    }
}
