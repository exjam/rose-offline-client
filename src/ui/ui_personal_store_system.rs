use bevy::{
    math::Vec3Swizzles,
    prelude::{
        Assets, Entity, EventReader, EventWriter, Events, Local, Query, Res, ResMut, With, World,
    },
};
use bevy_egui::{egui, EguiContext};
use rose_data::Item;
use rose_game_common::{
    components::Money,
    messages::client::{ClientMessage, PersonalStoreBuyItem},
};

use crate::{
    components::{ClientEntity, PersonalStore, PlayerCharacter, Position},
    events::{MessageBoxEvent, PersonalStoreEvent},
    resources::{GameConnection, GameData, UiResources, UiSpriteSheetType},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem},
        ui_add_item_tooltip,
        widgets::{DataBindings, Dialog},
        DragAndDropId, DragAndDropSlot, UiStateDragAndDrop,
    },
};

use super::widgets::DrawText;

const IID_BTN_CLOSE: i32 = 20;
const IID_RADIOBOX: i32 = 30;
const IID_BTN_SELL: i32 = 31;
const IID_BTN_BUY: i32 = 32;

pub struct UiPersonalStoreState {
    store_owner: Option<Entity>,
    store_sell_items: [Option<(Item, Money)>; 30],
    store_buy_items: [Option<(Item, Money)>; 30],
    selected_tab: i32,
}

impl Default for UiPersonalStoreState {
    fn default() -> Self {
        Self {
            store_owner: None,
            store_sell_items: Default::default(),
            store_buy_items: Default::default(),
            selected_tab: IID_BTN_SELL,
        }
    }
}

fn ui_add_store_item_slot(
    ui: &mut egui::Ui,
    ui_state_dnd: &mut UiStateDragAndDrop,
    dnd_id: DragAndDropId,
    pos: egui::Pos2,
    item: &Item,
    price: &Money,
    is_sell_item: bool,
    slot_index: usize,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    message_box_events: &mut EventWriter<MessageBoxEvent>,
) {
    let item_data = game_data.items.get_base_item(item.get_item_reference());
    let sprite = item_data.and_then(|item_data| {
        ui_resources.get_sprite_by_index(UiSpriteSheetType::Item, item_data.icon_index as usize)
    });
    let quantity = if item.get_item_type().is_stackable_item() {
        Some(item.get_quantity() as usize)
    } else {
        None
    };

    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::new(
                        dnd_id,
                        sprite,
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

    if is_sell_item && response.double_clicked() {
        let item = item.clone();

        message_box_events.send(MessageBoxEvent::Show {
            message: format!(
                "Are you sure you want to buy {} for {} Zuly?",
                item_data.as_ref().map(|x| x.name).unwrap_or(""),
                price.0
            ),
            modal: false,
            ok: Some(Box::new(move |commands| {
                commands.add(move |world: &mut World| {
                    if let Some(mut personal_store_events) =
                        world.get_resource_mut::<Events<PersonalStoreEvent>>()
                    {
                        personal_store_events
                            .send(PersonalStoreEvent::BuyItem { slot_index, item });
                    }
                });
            })),
            cancel: Some(Box::new(|_| {})),
        });
    }

    response.on_hover_ui(|ui| {
        ui_add_item_tooltip(ui, game_data, player_tooltip_data, item);

        if is_sell_item {
            ui.colored_label(egui::Color32::YELLOW, format!("Price: {}", price.0));
        } else {
            ui.colored_label(egui::Color32::GREEN, format!("Price: {}", price.0));
        }
    });
}

pub fn ui_personal_store_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiPersonalStoreState>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut personal_store_events: EventReader<PersonalStoreEvent>,
    query_personal_store: Query<(&ClientEntity, &PersonalStore, &Position), With<PersonalStore>>,
    query_player: Query<&Position, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    mut message_box_events: EventWriter<MessageBoxEvent>,
) {
    let ui_state = &mut *ui_state;

    for event in personal_store_events.iter() {
        match event {
            &PersonalStoreEvent::OpenEntityStore(entity) => {
                // Close previous store
                *ui_state = Default::default();

                // Open new store and request item list
                if let Ok((client_entity, _, _)) = query_personal_store.get(entity) {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::PersonalStoreListItems(client_entity.id))
                            .ok();
                    }

                    ui_state.store_owner = Some(entity);
                }
            }
            PersonalStoreEvent::SetItemList(item_list) => {
                ui_state.store_buy_items.fill(None);
                ui_state.store_sell_items.fill(None);

                for (slot_index, item, price) in item_list.buy_items.iter() {
                    if let Some(store_slot) = ui_state.store_buy_items.get_mut(*slot_index as usize)
                    {
                        *store_slot = Some((item.clone(), *price));
                    }
                }

                for (slot_index, item, price) in item_list.sell_items.iter() {
                    if let Some(store_slot) =
                        ui_state.store_sell_items.get_mut(*slot_index as usize)
                    {
                        *store_slot = Some((item.clone(), *price));
                    }
                }
            }
            PersonalStoreEvent::BuyItem { slot_index, item } => {
                if let Some((store_client_entity, _, _)) = ui_state
                    .store_owner
                    .and_then(|entity| query_personal_store.get(entity).ok())
                {
                    if let Some(game_connection) = &game_connection {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::PersonalStoreBuyItem(PersonalStoreBuyItem {
                                store_entity_id: store_client_entity.id,
                                store_slot_index: *slot_index,
                                buy_item: item.clone(),
                            }))
                            .ok();
                    }
                }
            }
            PersonalStoreEvent::UpdateBuyList { entity, item_list } => {
                if ui_state.store_owner == Some(*entity) {
                    for (slot_index, item) in item_list.iter() {
                        if let Some(store_slot) =
                            ui_state.store_buy_items.get_mut(*slot_index as usize)
                        {
                            if item.is_none() {
                                *store_slot = None;
                            } else if let Some((store_slot_item, _)) = store_slot.as_mut() {
                                *store_slot_item = item.as_ref().unwrap().clone();
                            }
                        }
                    }
                }
            }
            PersonalStoreEvent::UpdateSellList { entity, item_list } => {
                if ui_state.store_owner == Some(*entity) {
                    for (slot_index, item) in item_list.iter() {
                        if let Some(store_slot) =
                            ui_state.store_sell_items.get_mut(*slot_index as usize)
                        {
                            if item.is_none() {
                                *store_slot = None;
                            } else if let Some((store_slot_item, _)) = store_slot.as_mut() {
                                *store_slot_item = item.as_ref().unwrap().clone();
                            }
                        }
                    }
                }
            }
        }
    }

    let personal_store_entity = if let Some(entity) = ui_state.store_owner {
        entity
    } else {
        return;
    };

    let (_personal_store_client_entity, personal_store, personal_store_position) =
        if let Ok(personal_store) = query_personal_store.get(personal_store_entity) {
            personal_store
        } else {
            *ui_state = Default::default();
            return;
        };

    // Ensure player still in distance of personal store
    if let Ok(player_position) = query_player.get_single() {
        if player_position
            .position
            .xy()
            .distance(personal_store_position.position.xy())
            > 1100.0
        {
            *ui_state = Default::default();
            return;
        }
    }

    if ui_state.store_owner.is_none() {
        return;
    }

    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_personal_store) {
        dialog
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    let mut response_close_button = None;

    egui::Window::new("Personal Store")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    radio: &mut [(IID_RADIOBOX, &mut ui_state.selected_tab)],
                    response: &mut [(IID_BTN_CLOSE, &mut response_close_button)],
                    ..Default::default()
                },
                |ui, bindings| {
                    ui.add_label_at(egui::pos2(35.0, 6.0), &personal_store.title);

                    match bindings.get_radio(IID_RADIOBOX) {
                        Some(&mut IID_BTN_SELL) => {
                            for y in 0..6 {
                                for x in 0..5 {
                                    let slot_index = (y * 5 + x) as usize;
                                    if let Some((item, price)) =
                                        ui_state.store_sell_items[(y * 5 + x) as usize].as_ref()
                                    {
                                        ui_add_store_item_slot(
                                            ui,
                                            &mut ui_state_dnd,
                                            DragAndDropId::PersonalStoreSell(slot_index),
                                            egui::pos2(
                                                10.0 + x as f32 * 41.0,
                                                54.0 + y as f32 * 41.0,
                                            ),
                                            item,
                                            price,
                                            true,
                                            slot_index,
                                            player_tooltip_data.as_ref(),
                                            &game_data,
                                            &ui_resources,
                                            &mut message_box_events,
                                        );
                                    }
                                }
                            }
                        }
                        Some(&mut IID_BTN_BUY) => {
                            for y in 0..6 {
                                for x in 0..5 {
                                    let slot_index = (y * 5 + x) as usize;
                                    if let Some((item, price)) =
                                        ui_state.store_buy_items[slot_index].as_ref()
                                    {
                                        ui_add_store_item_slot(
                                            ui,
                                            &mut ui_state_dnd,
                                            DragAndDropId::NotDraggable,
                                            egui::pos2(
                                                10.0 + x as f32 * 41.0,
                                                54.0 + y as f32 * 41.0,
                                            ),
                                            item,
                                            price,
                                            false,
                                            slot_index,
                                            player_tooltip_data.as_ref(),
                                            &game_data,
                                            &ui_resources,
                                            &mut message_box_events,
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    };
                },
            );
        });

    if response_close_button.map_or(false, |x| x.clicked()) {
        *ui_state = Default::default();
    }
}
