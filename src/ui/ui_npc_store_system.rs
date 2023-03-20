use bevy::{
    ecs::query::WorldQuery,
    math::Vec3Swizzles,
    prelude::{
        Assets, Entity, EventReader, EventWriter, Events, Local, Query, Res, ResMut, With, World,
    },
};
use bevy_egui::{egui, EguiContexts};

use rose_data::{Item, NpcData, NpcStoreTabData, NpcStoreTabId};
use rose_game_common::{
    components::{AbilityValues, Inventory, ItemSlot, Npc},
    messages::{
        client::{ClientMessage, NpcStoreBuyItem, NpcStoreTransaction},
        ClientEntityId,
    },
};

use crate::{
    components::{PlayerCharacter, Position},
    events::{MessageBoxEvent, NpcStoreEvent, NumberInputDialogEvent},
    resources::{
        ClientEntityList, GameConnection, GameData, UiResources, UiSpriteSheetType, WorldRates,
    },
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem},
        ui_add_item_tooltip,
        ui_drag_and_drop_system::UiStateDragAndDrop,
        widgets::{DataBindings, Dialog, DrawText},
        DragAndDropId, DragAndDropSlot,
    },
};

const NUM_BUY_ITEMS: usize = 10;
const NUM_BUY_ITEMS_PER_ROW: usize = 5;

const NUM_SELL_ITEMS: usize = 10;
const NUM_SELL_ITEMS_PER_ROW: usize = 5;

const IID_STORE_BTN_CLOSE: i32 = 20;
const IID_STORE_RADIOBOX: i32 = 30;
const IID_STORE_BTN_TAB1: i32 = 31;
const IID_STORE_BTN_TAB2: i32 = 32;
const IID_STORE_BTN_TAB3: i32 = 33;
const IID_STORE_BTN_TAB4: i32 = 34;

const IID_TRANSACTION_CANCEL: i32 = 10;
const IID_TRANSACTION_OK: i32 = 11;

struct PendingBuyItem {
    store_tab_index: usize,
    store_tab_slot: usize,
    quantity: usize,
}

struct PendingSellItem {
    item_slot: ItemSlot,
    quantity: usize,
}

pub struct UiNpcStoreState {
    owner_entity: Option<(Entity, ClientEntityId)>,
    current_tab_index: i32,
    store_tabs: [Option<(NpcStoreTabId, String)>; 4],
    buy_list: [Option<PendingBuyItem>; NUM_BUY_ITEMS],
    sell_list: [Option<PendingSellItem>; NUM_SELL_ITEMS],
}

impl Default for UiNpcStoreState {
    fn default() -> Self {
        Self {
            owner_entity: None,
            current_tab_index: IID_STORE_BTN_TAB1,
            store_tabs: Default::default(),
            buy_list: Default::default(),
            sell_list: Default::default(),
        }
    }
}

fn ui_add_store_item_slot(
    ui: &mut egui::Ui,
    ui_state_dnd: &mut UiStateDragAndDrop,
    pos: egui::Pos2,
    store_tab: Option<&NpcStoreTabData>,
    store_tab_index: usize,
    store_tab_slot: usize,
    buy_list: &mut [Option<PendingBuyItem>; NUM_BUY_ITEMS],
    player: Option<&NpcStorePlayerWorldQueryItem>,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    world_rates: Option<&Res<WorldRates>>,
    number_input_dialog_events: &mut EventWriter<NumberInputDialogEvent>,
) {
    let item_reference =
        store_tab.and_then(|store_tab| store_tab.items.get(&(store_tab_slot as u16)));
    let item_data =
        item_reference.and_then(|item_reference| game_data.items.get_base_item(*item_reference));
    let item = item_data.and_then(|item_data| Item::from_item_data(item_data, 999));
    let sprite = item_data.and_then(|item_data| {
        ui_resources.get_sprite_by_index(UiSpriteSheetType::Item, item_data.icon_index as usize)
    });
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
    } else {
        0
    };

    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::new(
                        DragAndDropId::NpcStore(store_tab_index, store_tab_slot),
                        sprite,
                        None,
                        false,
                        quantity,
                        None,
                        |_| false,
                        &mut ui_state_dnd.dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if let Some(item) = item.as_ref() {
        if response.double_clicked() {
            if item.is_stackable_item() {
                number_input_dialog_events.send(NumberInputDialogEvent::Show {
                    max_value: Some(999),
                    modal: false,
                    ok: Some(Box::new(move |commands, quantity| {
                        commands.add(move |world: &mut World| {
                            let mut npc_store_events =
                                world.resource_mut::<Events<NpcStoreEvent>>();
                            npc_store_events.send(NpcStoreEvent::AddToBuyList {
                                store_tab_index,
                                store_tab_slot,
                                quantity,
                            })
                        });
                    })),
                    cancel: None,
                });
            } else {
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
        }

        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, player_tooltip_data, item);

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
    pos: egui::Pos2,
    npc_data: &NpcData,
    buy_list: &mut [Option<PendingBuyItem>; NUM_BUY_ITEMS],
    buy_slot_index: usize,
    player: Option<&NpcStorePlayerWorldQueryItem>,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
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
    let sprite = item_data.and_then(|item_data| {
        ui_resources.get_sprite_by_index(UiSpriteSheetType::Item, item_data.icon_index as usize)
    });
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
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::new(
                        DragAndDropId::NpcStoreBuyList(buy_slot_index),
                        sprite,
                        None,
                        false,
                        quantity,
                        None,
                        buy_slot_drag_accepts,
                        &mut ui_state_dnd.dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if response.double_clicked() {
        *pending_buy_item = None;
    }

    if let Some(item) = item {
        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, player_tooltip_data, &item);

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
    pos: egui::Pos2,
    sell_list: &mut [Option<PendingSellItem>; NUM_SELL_ITEMS],
    sell_slot_index: usize,
    player: Option<&NpcStorePlayerWorldQueryItem>,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    world_rates: Option<&Res<WorldRates>>,
) -> i64 {
    let pending_sell_item = &mut sell_list[sell_slot_index];
    let item = player.and_then(|player| {
        pending_sell_item
            .as_ref()
            .and_then(|pending_sell_item| player.inventory.get_item(pending_sell_item.item_slot))
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
            * item.get_quantity() as i64
    } else {
        0
    };

    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::with_item(
                        DragAndDropId::NpcStoreSellList(sell_slot_index),
                        item,
                        None,
                        game_data,
                        ui_resources,
                        sell_slot_drag_accepts,
                        &mut ui_state_dnd.dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if response.double_clicked() {
        *pending_sell_item = None;
    }

    if let Some(item) = item {
        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, player_tooltip_data, item);

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
    mut egui_context: EguiContexts,
    mut ui_state: Local<UiNpcStoreState>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut npc_store_events: EventReader<NpcStoreEvent>,
    query_player: Query<NpcStorePlayerWorldQuery>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    query_npc: Query<NpcStoreNpcWorldQuery>,
    client_entity_list: Res<ClientEntityList>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    dialog_assets: Res<Assets<Dialog>>,
    ui_resources: Res<UiResources>,
    world_rates: Option<Res<WorldRates>>,
    mut number_input_dialog_events: EventWriter<NumberInputDialogEvent>,
    mut message_box_events: EventWriter<MessageBoxEvent>,
) {
    let ui_state = &mut *ui_state;
    let store_dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_npc_store) {
        dialog
    } else {
        return;
    };
    let transaction_dialog =
        if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_npc_transaction) {
            dialog
        } else {
            return;
        };

    for event in npc_store_events.iter() {
        match *event {
            NpcStoreEvent::OpenClientEntityStore(client_entity_id) => {
                *ui_state = UiNpcStoreState::default();

                if let Some(owner_entity) = client_entity_list.get(client_entity_id) {
                    if let Ok(npc) = query_npc.get(owner_entity) {
                        if let Some(npc_data) = game_data.npcs.get_npc(npc.npc.id) {
                            for (index, id) in npc_data.store_tabs.iter().enumerate() {
                                if let Some(id) = id {
                                    if let Some(store_tab) = game_data.npcs.get_store_tab(*id) {
                                        ui_state.store_tabs[index] =
                                            Some((*id, store_tab.name.to_string()));
                                    }
                                }
                            }

                            ui_state.owner_entity = Some((owner_entity, client_entity_id));
                        }
                    }
                }
            }
            NpcStoreEvent::AddToBuyList {
                store_tab_index,
                store_tab_slot,
                quantity,
            } => {
                for slot in ui_state.buy_list.iter_mut() {
                    if slot.is_none() {
                        *slot = Some(PendingBuyItem {
                            store_tab_index,
                            store_tab_slot,
                            quantity,
                        });
                        break;
                    }
                }
            }
            NpcStoreEvent::RemoveFromBuyList(index) => {
                if let Some(buy_slot) = ui_state.buy_list.get_mut(index) {
                    buy_slot.take();
                }
            }
            NpcStoreEvent::RemoveFromSellList(index) => {
                if let Some(buy_slot) = ui_state.sell_list.get_mut(index) {
                    buy_slot.take();
                }
            }
        }
    }

    let player = query_player.get_single().ok();
    let player_tooltip_data = query_player_tooltip.get_single().ok();
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

    let screen_size = egui_context
        .ctx_mut()
        .input(|input| input.screen_rect().size());

    let mut response_close = None;
    let mut response_cancel = None;
    let mut response_ok = None;

    egui::Window::new("NPC Store")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_pos([
            screen_size.x / 2.0 + 5.0,
            (screen_size.y - store_dialog.height) / 2.0,
        ])
        .default_size([store_dialog.width, store_dialog.height])
        .show(egui_context.ctx_mut(), |ui| {
            store_dialog.draw(
                ui,
                DataBindings {
                    response: &mut [(IID_STORE_BTN_CLOSE, &mut response_close)],
                    radio: &mut [(IID_STORE_RADIOBOX, &mut ui_state.current_tab_index)],
                    visible: &mut [
                        (IID_STORE_BTN_TAB1, ui_state.store_tabs[0].is_some()),
                        (IID_STORE_BTN_TAB2, ui_state.store_tabs[1].is_some()),
                        (IID_STORE_BTN_TAB3, ui_state.store_tabs[2].is_some()),
                        (IID_STORE_BTN_TAB4, ui_state.store_tabs[3].is_some()),
                    ],
                    label: &mut [
                        (
                            IID_STORE_BTN_TAB1,
                            ui_state.store_tabs[0].as_ref().map_or("", |x| x.1.as_str()),
                        ),
                        (
                            IID_STORE_BTN_TAB2,
                            ui_state.store_tabs[1].as_ref().map_or("", |x| x.1.as_str()),
                        ),
                        (
                            IID_STORE_BTN_TAB3,
                            ui_state.store_tabs[2].as_ref().map_or("", |x| x.1.as_str()),
                        ),
                        (
                            IID_STORE_BTN_TAB4,
                            ui_state.store_tabs[3].as_ref().map_or("", |x| x.1.as_str()),
                        ),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    let current_tab_index = match bindings.get_radio(IID_STORE_RADIOBOX) {
                        Some(&mut IID_STORE_BTN_TAB1) => 0,
                        Some(&mut IID_STORE_BTN_TAB2) => 1,
                        Some(&mut IID_STORE_BTN_TAB3) => 2,
                        Some(&mut IID_STORE_BTN_TAB4) => 3,
                        _ => 0,
                    };
                    let store_tab_id = npc_data.store_tabs[current_tab_index];

                    if let Some(current_store_tab) =
                        store_tab_id.and_then(|id| game_data.npcs.get_store_tab(id))
                    {
                        for row in 0..6 {
                            for column in 0..8 {
                                ui_add_store_item_slot(
                                    ui,
                                    ui_state_dnd.as_mut(),
                                    egui::pos2(
                                        11.0 + column as f32 * 41.0,
                                        51.0 + row as f32 * 41.0,
                                    ),
                                    Some(current_store_tab),
                                    current_tab_index,
                                    column + row * 8,
                                    &mut ui_state.buy_list,
                                    player.as_ref(),
                                    player_tooltip_data.as_ref(),
                                    &game_data,
                                    &ui_resources,
                                    world_rates.as_ref(),
                                    &mut number_input_dialog_events,
                                );
                            }
                        }
                    }
                },
            );
        });

    let mut transaction_cost = 0;

    egui::Window::new("NPC Transaction")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_pos([
            screen_size.x / 2.0 - 5.0 - transaction_dialog.width,
            (screen_size.y - store_dialog.height) / 2.0,
        ])
        .default_size([transaction_dialog.width, transaction_dialog.height])
        .show(egui_context.ctx_mut(), |ui| {
            transaction_dialog.draw(
                ui,
                DataBindings {
                    response: &mut [
                        (IID_TRANSACTION_CANCEL, &mut response_cancel),
                        (IID_TRANSACTION_OK, &mut response_ok),
                    ],
                    ..Default::default()
                },
                |ui, _bindings| {
                    let mut buy_item_price = 0;
                    for i in 0..NUM_BUY_ITEMS {
                        let column = (i % NUM_BUY_ITEMS_PER_ROW) as f32;
                        let row = (i / NUM_BUY_ITEMS_PER_ROW) as f32;

                        buy_item_price += ui_add_buy_item_slot(
                            ui,
                            ui_state_dnd.as_mut(),
                            egui::pos2(10.0 + column * 41.0, 52.0 + row * 41.0),
                            npc_data,
                            &mut ui_state.buy_list,
                            i,
                            player.as_ref(),
                            player_tooltip_data.as_ref(),
                            &game_data,
                            &ui_resources,
                            world_rates.as_ref(),
                        );
                    }
                    ui.add_label_at(egui::pos2(39.0, 139.0), format!("{}", buy_item_price));
                    transaction_cost += buy_item_price;

                    let mut sell_item_value = 0;
                    for i in 0..NUM_SELL_ITEMS {
                        let column = (i % NUM_SELL_ITEMS_PER_ROW) as f32;
                        let row = (i / NUM_SELL_ITEMS_PER_ROW) as f32;

                        sell_item_value += ui_add_sell_item_slot(
                            ui,
                            ui_state_dnd.as_mut(),
                            egui::pos2(10.0 + column * 41.0, 183.0 + row * 41.0),
                            &mut ui_state.sell_list,
                            i,
                            player.as_ref(),
                            player_tooltip_data.as_ref(),
                            &game_data,
                            &ui_resources,
                            world_rates.as_ref(),
                        );
                    }
                    ui.add_label_at(egui::pos2(39.0, 272.0), format!("{}", sell_item_value));
                    transaction_cost -= sell_item_value;
                },
            );
        });

    if response_ok.map_or(false, |x| x.clicked()) {
        let can_afford_transaction =
            player.map_or(true, |player| transaction_cost <= player.inventory.money.0);
        // TODO: Check inventory space

        if can_afford_transaction {
            let mut buy_items = Vec::new();
            let mut sell_items = Vec::new();

            for pending_buy_item in ui_state.buy_list.iter_mut().filter_map(|x| x.take()) {
                buy_items.push(NpcStoreBuyItem {
                    tab_index: pending_buy_item.store_tab_index,
                    item_index: pending_buy_item.store_tab_slot,
                    quantity: pending_buy_item.quantity,
                });
            }

            for pending_sell_item in ui_state.sell_list.iter_mut().filter_map(|x| x.take()) {
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
        } else {
            message_box_events.send(MessageBoxEvent::Show {
                message: "You do not have enough Zuly for this transaction.".to_string(),
                modal: true,
                ok: Some(Box::new(|_| {})),
                cancel: None,
            });
        }
    }

    if response_close.map_or(false, |x| x.clicked())
        || response_cancel.map_or(false, |x| x.clicked())
    {
        ui_state.owner_entity = None;
    }
}
