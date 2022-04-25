use bevy::{
    math::Vec3Swizzles,
    prelude::{Entity, EventReader, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

use rose_data::{Item, NpcData, NpcStoreTabData};
use rose_game_common::components::{Inventory, ItemSlot, Npc};

use crate::{
    components::{PlayerCharacter, Position},
    events::NpcStoreEvent,
    resources::{ClientEntityList, GameData, Icons},
    ui::{
        ui_add_item_tooltip, ui_drag_and_drop_system::UiStateDragAndDrop, DragAndDropId,
        DragAndDropSlot,
    },
};

const NUM_BUY_ITEMS: usize = 10;
const NUM_BUY_ITEMS_PER_ROW: usize = 5;

const NUM_SELL_ITEMS: usize = 10;
const NUM_SELL_ITEMS_PER_ROW: usize = 5;

struct PendingBuyItem {
    store_tab_index: usize,
    store_tab_slot: usize,
    quantity: usize,
}

struct PendingSellItem {
    item_slot: ItemSlot,
    quantity: usize,
}

#[derive(Default)]
pub struct UiNpcStoreState {
    owner_entity: Option<Entity>,
    buy_list: [Option<PendingBuyItem>; NUM_BUY_ITEMS],
    sell_list: [Option<PendingSellItem>; NUM_SELL_ITEMS],
}

fn ui_add_store_item_slot(
    ui: &mut egui::Ui,
    ui_state_dnd: &mut UiStateDragAndDrop,
    store_tab: Option<&NpcStoreTabData>,
    store_tab_index: usize,
    store_tab_slot: usize,
    buy_list: &mut [Option<PendingBuyItem>; NUM_BUY_ITEMS],
    game_data: &GameData,
    icons: &Icons,
) {
    let item_reference =
        store_tab.and_then(|store_tab| store_tab.items.get(&(store_tab_slot as u16)));
    let item_data =
        item_reference.and_then(|item_reference| game_data.items.get_base_item(*item_reference));
    let item = item_data.and_then(|item_data| Item::from_item_data(item_data, 999));
    let contents =
        item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize));
    let quantity = item.as_ref().and_then(|item| {
        if item.get_item_type().is_stackable_item() {
            Some(item.get_quantity() as usize)
        } else {
            None
        }
    });

    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::NpcStore(store_tab_index, store_tab_slot),
        contents,
        quantity,
        None,
        |_| false,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if let Some(item) = item.as_ref() {
        if response.double_clicked() {
            for slot in buy_list.iter_mut() {
                if slot.is_none() {
                    *slot = Some(PendingBuyItem {
                        store_tab_index,
                        store_tab_slot,
                        quantity: 1,
                    });
                    break;
                }
            }
        }

        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, item);
        });
    }
}

fn buy_slot_drag_accepts(drag_source: &DragAndDropId) -> bool {
    matches!(drag_source, DragAndDropId::NpcStore(_, _))
}

fn sell_slot_drag_accepts(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(ItemSlot::Inventory(_, _))
    )
}

fn ui_add_buy_item_slot(
    ui: &mut egui::Ui,
    ui_state_dnd: &mut UiStateDragAndDrop,
    npc_data: &NpcData,
    buy_list: &mut [Option<PendingBuyItem>; NUM_BUY_ITEMS],
    buy_slot_index: usize,
    game_data: &GameData,
    icons: &Icons,
) {
    let pending_buy_item = &mut buy_list[buy_slot_index];
    let item_reference = pending_buy_item.as_ref().and_then(|pending_buy_item| {
        npc_data
            .store_tabs
            .get(pending_buy_item.store_tab_index)
            .and_then(|x| x.as_ref())
            .and_then(|store_tab| game_data.npcs.get_store_tab(*store_tab))
            .and_then(|store_tab| {
                store_tab
                    .items
                    .get(&(pending_buy_item.store_tab_slot as u16))
            })
    });
    let item_data =
        item_reference.and_then(|item_reference| game_data.items.get_base_item(*item_reference));
    let item = item_data.and_then(|item_data| Item::from_item_data(item_data, 999));
    let contents =
        item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize));
    let quantity = item.as_ref().and_then(|item| {
        if item.get_item_type().is_stackable_item() {
            Some(pending_buy_item.as_ref().unwrap().quantity)
        } else {
            None
        }
    });

    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::NpcStoreBuyList(buy_slot_index),
        contents,
        quantity,
        None,
        buy_slot_drag_accepts,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if response.double_clicked() {
        *pending_buy_item = None;
    }

    if let Some(DragAndDropId::NpcStore(store_tab_index, store_tab_slot)) = dropped_item {
        *pending_buy_item = Some(PendingBuyItem {
            store_tab_index,
            store_tab_slot,
            quantity: 1,
        });
    }
}

fn ui_add_sell_item_slot(
    ui: &mut egui::Ui,
    ui_state_dnd: &mut UiStateDragAndDrop,
    sell_list: &mut [Option<PendingSellItem>; NUM_SELL_ITEMS],
    sell_slot_index: usize,
    inventory: Option<&Inventory>,
    game_data: &GameData,
    icons: &Icons,
) {
    let pending_sell_item = &mut sell_list[sell_slot_index];
    let item = pending_sell_item.as_ref().and_then(|pending_sell_item| {
        inventory.and_then(|inventory| inventory.get_item(pending_sell_item.item_slot))
    });
    let item_data = item.and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
    let contents =
        item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize));
    let quantity = item.as_ref().and_then(|item| {
        if item.get_item_type().is_stackable_item() {
            Some(pending_sell_item.as_ref().unwrap().quantity)
        } else {
            None
        }
    });

    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::NpcStoreSellList(sell_slot_index),
        contents,
        quantity,
        None,
        sell_slot_drag_accepts,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if response.double_clicked() {
        *pending_sell_item = None;
    }

    if let Some(DragAndDropId::Inventory(item_slot)) = dropped_item {
        *pending_sell_item = Some(PendingSellItem {
            item_slot,
            quantity: 1,
        });
    }
}

pub fn ui_npc_store_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiNpcStoreState>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut npc_store_events: EventReader<NpcStoreEvent>,
    query_player_position: Query<&Position, With<PlayerCharacter>>,
    query_npc: Query<&Npc>,
    query_npc_position: Query<&Position>,
    query_inventory: Query<&Inventory, With<PlayerCharacter>>,
    client_entity_list: Res<ClientEntityList>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    for event in npc_store_events.iter() {
        let NpcStoreEvent::OpenClientEntityStore(client_entity_id) = event;
        *ui_state = UiNpcStoreState {
            owner_entity: client_entity_list.get(*client_entity_id),
            ..Default::default()
        };
    }

    // If player has moved away from NPC, close the dialog
    if let (Ok(player_position), Some(npc_position)) = (
        query_player_position.get_single(),
        ui_state
            .owner_entity
            .and_then(|entity| query_npc_position.get(entity).ok()),
    ) {
        if npc_position
            .position
            .xy()
            .distance(player_position.position.xy())
            > 400.0
        {
            ui_state.owner_entity = None;
            return;
        }
    }

    let npc_data = ui_state
        .owner_entity
        .and_then(|owner_entity| query_npc.get(owner_entity).ok())
        .and_then(|npc| game_data.npcs.get_npc(npc.id));
    if npc_data.is_none() {
        return;
    }
    let npc_data = npc_data.unwrap();
    let inventory = query_inventory.get_single().ok();

    let mut is_open = true;
    egui::Window::new(&npc_data.name)
        .id(egui::Id::new("npc_store_window"))
        .resizable(false)
        .open(&mut is_open)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                egui::ScrollArea::vertical()
                    .min_scrolled_height(400.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for (store_tab_index, store_tab_id) in
                                npc_data.store_tabs.iter().enumerate()
                            {
                                if let Some(current_store_tab) =
                                    store_tab_id.and_then(|id| game_data.npcs.get_store_tab(id))
                                {
                                    ui.group(|ui| {
                                        ui.label(&current_store_tab.name);

                                        egui::Grid::new("store_items_grid")
                                            .num_columns(8)
                                            .spacing([2.0, 2.0])
                                            .show(ui, |ui| {
                                                for row in 0..6 {
                                                    for column in 0..8 {
                                                        ui_add_store_item_slot(
                                                            ui,
                                                            ui_state_dnd.as_mut(),
                                                            Some(current_store_tab),
                                                            store_tab_index,
                                                            column + row * 8,
                                                            &mut ui_state.buy_list,
                                                            &game_data,
                                                            &icons,
                                                        );
                                                    }

                                                    ui.end_row();
                                                }
                                            });
                                    });
                                }
                            }
                        });
                    });

                ui.vertical(|ui| {
                    ui.group(|ui| {
                        ui.label("Buy Items");
                        egui::Grid::new("buy_items_grid")
                            .num_columns(NUM_BUY_ITEMS_PER_ROW)
                            .spacing([2.0, 2.0])
                            .show(ui, |ui| {
                                for i in 0..NUM_BUY_ITEMS {
                                    if i != 0 && (i % NUM_BUY_ITEMS_PER_ROW) == 0 {
                                        ui.end_row();
                                    }

                                    ui_add_buy_item_slot(
                                        ui,
                                        ui_state_dnd.as_mut(),
                                        npc_data,
                                        &mut ui_state.buy_list,
                                        i,
                                        &game_data,
                                        &icons,
                                    );
                                }
                            });
                    });

                    ui.group(|ui| {
                        ui.label("Sell Items");
                        egui::Grid::new("sell_items_grid")
                            .num_columns(NUM_SELL_ITEMS_PER_ROW)
                            .spacing([2.0, 2.0])
                            .show(ui, |ui| {
                                for i in 0..NUM_SELL_ITEMS {
                                    if i != 0 && (i % NUM_SELL_ITEMS_PER_ROW) == 0 {
                                        ui.end_row();
                                    }

                                    ui_add_sell_item_slot(
                                        ui,
                                        ui_state_dnd.as_mut(),
                                        &mut ui_state.sell_list,
                                        i,
                                        inventory,
                                        &game_data,
                                        &icons,
                                    );
                                }
                            });
                    });

                    // TODO: Calculate price of buy items - sell items
                    ui.label("Total Price: 0 Zuly");

                    ui.horizontal(|ui| {
                        if ui.button("Buy").clicked() {
                            log::warn!("TODO: Implement buy from NPC store");
                        }

                        if ui.button("Cancel").clicked() {
                            ui_state.owner_entity = None;
                        }
                    })
                });
            });
        });

    if !is_open {
        ui_state.owner_entity = None;
    }
}
