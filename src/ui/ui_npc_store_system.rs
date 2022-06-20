use bevy::{
    ecs::query::WorldQuery,
    math::Vec3Swizzles,
    prelude::{Entity, EventReader, Local, Query, Res, ResMut},
};
use bevy_egui::{egui, EguiContext};

use rose_data::{Item, NpcData, NpcStoreTabData};
use rose_game_common::{
    components::{AbilityValues, Inventory, ItemSlot, Npc},
    messages::{
        client::{ClientMessage, NpcStoreBuyItem, NpcStoreTransaction},
        ClientEntityId,
    },
};

use crate::{
    components::{PlayerCharacter, Position},
    events::NpcStoreEvent,
    resources::{ClientEntityList, GameConnection, GameData, Icons, WorldRates},
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
    owner_entity: Option<(Entity, ClientEntityId)>,
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
    player: Option<&NpcStorePlayerWorldQueryItem>,
    game_data: &GameData,
    icons: &Icons,
    world_rates: Option<&Res<WorldRates>>,
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

    let item_price = if let Some(item_reference) = item_reference {
        game_data
            .ability_value_calculator
            .calculate_npc_store_item_buy_price(
                &game_data.items,
                *item_reference,
                player.map_or(0, |x| x.ability_values.get_npc_store_buy_rate()),
                world_rates.map_or(100, |x| x.item_price_rate),
                world_rates.map_or(100, |x| x.town_price_rate),
            )
            .unwrap_or(0) as i64
            * quantity.unwrap_or(1) as i64
    } else {
        0
    };

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

            ui.colored_label(egui::Color32::YELLOW, format!("Buy Price: {}", item_price));
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
    player: Option<&NpcStorePlayerWorldQueryItem>,
    game_data: &GameData,
    icons: &Icons,
    world_rates: Option<&Res<WorldRates>>,
) -> i64 {
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

    let item_price = if let Some(item_reference) = item_reference {
        game_data
            .ability_value_calculator
            .calculate_npc_store_item_buy_price(
                &game_data.items,
                *item_reference,
                player.map_or(0, |player| player.ability_values.get_npc_store_buy_rate()),
                world_rates.map_or(100, |x| x.item_price_rate),
                world_rates.map_or(100, |x| x.town_price_rate),
            )
            .unwrap_or(0) as i64
            * quantity.unwrap_or(1) as i64
    } else {
        0
    };

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

    if let Some(item) = item {
        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, &item);

            ui.colored_label(egui::Color32::YELLOW, format!("Buy Price: {}", item_price));
        });
    }

    if let Some(DragAndDropId::NpcStore(store_tab_index, store_tab_slot)) = dropped_item {
        *pending_buy_item = Some(PendingBuyItem {
            store_tab_index,
            store_tab_slot,
            quantity: 1,
        });
    }

    item_price
}

fn ui_add_sell_item_slot(
    ui: &mut egui::Ui,
    ui_state_dnd: &mut UiStateDragAndDrop,
    sell_list: &mut [Option<PendingSellItem>; NUM_SELL_ITEMS],
    sell_slot_index: usize,
    player: Option<&NpcStorePlayerWorldQueryItem>,
    game_data: &GameData,
    icons: &Icons,
    world_rates: Option<&Res<WorldRates>>,
) -> i64 {
    let pending_sell_item = &mut sell_list[sell_slot_index];
    let item = player.and_then(|player| {
        pending_sell_item
            .as_ref()
            .and_then(|pending_sell_item| player.inventory.get_item(pending_sell_item.item_slot))
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

    let item_price = if let Some(item) = item {
        game_data
            .ability_value_calculator
            .calculate_npc_store_item_sell_price(
                &game_data.items,
                item,
                player.map_or(0, |player| player.ability_values.get_npc_store_sell_rate()),
                world_rates.map_or(0, |x| x.world_price_rate),
                world_rates.map_or(0, |x| x.item_price_rate),
                world_rates.map_or(0, |x| x.town_price_rate),
            )
            .unwrap_or(0) as i64
            * quantity.unwrap_or(1) as i64
    } else {
        0
    };

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

    if let Some(item) = item {
        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, item);

            ui.colored_label(egui::Color32::YELLOW, format!("Sell Value: {}", item_price));
        });
    }

    if let Some(DragAndDropId::Inventory(item_slot)) = dropped_item {
        *pending_sell_item = Some(PendingSellItem {
            item_slot,
            quantity: 1,
        });
    }

    item_price
}

#[derive(WorldQuery)]
pub struct NpcStorePlayerWorldQuery<'w> {
    ability_values: &'w AbilityValues,
    inventory: &'w Inventory,
    position: &'w Position,
    player_character: &'w PlayerCharacter,
}

#[derive(WorldQuery)]
pub struct NpcStoreNpcWorldQuery<'w> {
    npc: &'w Npc,
    position: &'w Position,
}

pub fn ui_npc_store_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiNpcStoreState>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut npc_store_events: EventReader<NpcStoreEvent>,
    query_player: Query<NpcStorePlayerWorldQuery>,
    query_npc: Query<NpcStoreNpcWorldQuery>,
    client_entity_list: Res<ClientEntityList>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
    world_rates: Option<Res<WorldRates>>,
) {
    for event in npc_store_events.iter() {
        let &NpcStoreEvent::OpenClientEntityStore(client_entity_id) = event;
        *ui_state = UiNpcStoreState {
            owner_entity: client_entity_list
                .get(client_entity_id)
                .map(|entity| (entity, client_entity_id)),
            ..Default::default()
        };
    }

    let player = query_player.get_single().ok();
    let npc = ui_state
        .owner_entity
        .and_then(|(owner_entity, _)| query_npc.get(owner_entity).ok());

    // If player has moved away from NPC, close the dialog
    if let (Some(player), Some(npc)) = (player.as_ref(), npc.as_ref()) {
        if player.position.position.xy().distance(npc.position.xy()) > 600.0 {
            ui_state.owner_entity = None;
            return;
        }
    }

    let npc_data = npc.and_then(|npc| game_data.npcs.get_npc(npc.npc.id));
    if npc_data.is_none() {
        return;
    }
    let npc_data = npc_data.unwrap();

    let mut is_open = true;
    egui::Window::new(&npc_data.name)
        .id(egui::Id::new("npc_store_window"))
        .resizable(false)
        .open(&mut is_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut transaction_cost = 0;

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
                                                            player.as_ref(),
                                                            &game_data,
                                                            &icons,
                                                            world_rates.as_ref(),
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
                        let mut buy_item_price = 0;
                        ui.label("Buy Items");

                        egui::Grid::new("buy_items_grid")
                            .num_columns(NUM_BUY_ITEMS_PER_ROW)
                            .spacing([2.0, 2.0])
                            .show(ui, |ui| {
                                for i in 0..NUM_BUY_ITEMS {
                                    if i != 0 && (i % NUM_BUY_ITEMS_PER_ROW) == 0 {
                                        ui.end_row();
                                    }

                                    buy_item_price += ui_add_buy_item_slot(
                                        ui,
                                        ui_state_dnd.as_mut(),
                                        npc_data,
                                        &mut ui_state.buy_list,
                                        i,
                                        player.as_ref(),
                                        &game_data,
                                        &icons,
                                        world_rates.as_ref(),
                                    );
                                }
                            });

                        ui.label(format!("Total Price: {}", buy_item_price));
                        transaction_cost += buy_item_price;
                    });

                    ui.group(|ui| {
                        let mut sell_item_value = 0;
                        ui.label("Sell Items");

                        egui::Grid::new("sell_items_grid")
                            .num_columns(NUM_SELL_ITEMS_PER_ROW)
                            .spacing([2.0, 2.0])
                            .show(ui, |ui| {
                                for i in 0..NUM_SELL_ITEMS {
                                    if i != 0 && (i % NUM_SELL_ITEMS_PER_ROW) == 0 {
                                        ui.end_row();
                                    }

                                    sell_item_value += ui_add_sell_item_slot(
                                        ui,
                                        ui_state_dnd.as_mut(),
                                        &mut ui_state.sell_list,
                                        i,
                                        player.as_ref(),
                                        &game_data,
                                        &icons,
                                        world_rates.as_ref(),
                                    );
                                }
                            });

                        ui.label(format!("Total Value: {}", sell_item_value));
                        transaction_cost -= sell_item_value;
                    });
                });
            });

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                let can_afford_transaction =
                    player.map_or(true, |player| transaction_cost <= player.inventory.money.0);

                if ui.button("Cancel").clicked() {
                    ui_state.owner_entity = None;
                }

                if ui
                    .add_enabled(can_afford_transaction, egui::Button::new("Buy"))
                    .clicked()
                {
                    let mut buy_items = Vec::new();
                    let mut sell_items = Vec::new();

                    for pending_buy_item in ui_state.buy_list.iter_mut().filter_map(|x| x.take()) {
                        buy_items.push(NpcStoreBuyItem {
                            tab_index: pending_buy_item.store_tab_index,
                            item_index: pending_buy_item.store_tab_slot,
                            quantity: pending_buy_item.quantity,
                        });
                    }

                    for pending_sell_item in ui_state.sell_list.iter_mut().filter_map(|x| x.take())
                    {
                        sell_items.push((pending_sell_item.item_slot, pending_sell_item.quantity));
                    }

                    if let Some(game_connection) = game_connection {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::NpcStoreTransaction(NpcStoreTransaction {
                                npc_entity_id: ui_state.owner_entity.unwrap().1,
                                buy_items,
                                sell_items,
                            }))
                            .ok();
                    }
                }

                ui.colored_label(
                    if transaction_cost < 0 {
                        egui::Color32::GREEN
                    } else if can_afford_transaction {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    },
                    format!("Transaction Cost: {} Zuly", transaction_cost),
                );
            });
        });

    if !is_open {
        ui_state.owner_entity = None;
    }
}
