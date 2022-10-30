use bevy::{
    ecs::query::WorldQuery,
    math::Vec3Swizzles,
    prelude::{Assets, Entity, EventReader, EventWriter, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

use rose_data::Item;
use rose_game_common::{
    components::{CharacterInfo, ItemSlot},
    messages::client::ClientMessage,
};

use crate::{
    components::{Bank, PlayerCharacter, Position},
    events::{BankEvent, PlayerCommandEvent},
    resources::{ClientEntityList, GameConnection, GameData, UiResources, UiSpriteSheetType},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem},
        ui_add_item_tooltip,
        widgets::{DataBindings, Dialog},
        DragAndDropId, DragAndDropSlot, UiStateDragAndDrop, UiStateWindows,
    },
};

const IID_BTN_CLOSE: i32 = 20;
const IID_RADIOBOX: i32 = 30;
const IID_BTN_TAB1: i32 = 31;
const IID_BTN_TAB2: i32 = 32;
const IID_BTN_TAB3: i32 = 33;
const IID_BTN_TAB4: i32 = 34;

const BANK_SLOTS_PER_PAGE: usize = 40;
const BANK_SLOTS_PER_ROW: usize = 8;

pub struct UiStateBank {
    bank_open: bool,
    bank_entity: Option<Entity>,
    current_page: i32,
}

impl Default for UiStateBank {
    fn default() -> Self {
        Self {
            bank_open: false,
            bank_entity: None,
            current_page: IID_BTN_TAB1,
        }
    }
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    bank: &'w Bank,
    character_info: &'w CharacterInfo,
    position: &'w Position,
}

fn ui_add_bank_slot(
    ui: &mut egui::Ui,
    bank_slot_index: usize,
    pos: egui::Pos2,
    player: &PlayerQueryItem,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    ui_state_dnd: &mut UiStateDragAndDrop,
    player_command_events: &mut EventWriter<PlayerCommandEvent>,
) {
    let item = player
        .bank
        .slots
        .get(bank_slot_index)
        .and_then(|x| x.as_ref());
    let item_data = item
        .as_ref()
        .and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
    let sprite = item_data.and_then(|item_data| {
        ui_resources.get_sprite_by_index(UiSpriteSheetType::Item, item_data.icon_index as usize)
    });
    let socket_sprite =
        item.as_ref()
            .and_then(|item| item.as_equipment())
            .and_then(|equipment_item| {
                if equipment_item.has_socket {
                    if equipment_item.gem > 300 {
                        let gem_item_data =
                            game_data.items.get_gem_item(equipment_item.gem as usize)?;
                        ui_resources.get_sprite_by_index(
                            UiSpriteSheetType::ItemSocketGem,
                            gem_item_data.gem_sprite_id as usize,
                        )
                    } else {
                        ui_resources.get_item_socket_sprite()
                    }
                } else {
                    None
                }
            });
    let broken = item
        .and_then(|item| item.as_equipment())
        .map_or(false, |item| item.life == 0);

    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::new(
                        DragAndDropId::Bank(bank_slot_index),
                        sprite,
                        socket_sprite,
                        broken,
                        match item.as_ref() {
                            Some(Item::Stackable(stackable_item)) => {
                                Some(stackable_item.quantity as usize)
                            }
                            _ => None,
                        },
                        None,
                        |drag_source: &DragAndDropId| -> bool {
                            matches!(
                                drag_source,
                                DragAndDropId::Inventory(ItemSlot::Inventory(_, _))
                            )
                        },
                        &mut ui_state_dnd.dragged_item,
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

    if let Some(DragAndDropId::Inventory(dropped_inventory_slot)) = dropped_item {
        player_command_events.send(PlayerCommandEvent::BankDepositItem(dropped_inventory_slot));
    }
}

pub fn ui_bank_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiStateBank>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    mut bank_events: EventReader<BankEvent>,
    client_entity_list: Res<ClientEntityList>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    query_position: Query<&Position>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_bank) {
        dialog
    } else {
        return;
    };

    for event in bank_events.iter() {
        match *event {
            BankEvent::OpenBankFromClientEntity { client_entity_id } => {
                if let Some(entity) = client_entity_list.get(client_entity_id) {
                    if let Some(game_connection) = game_connection.as_ref() {
                        ui_state.bank_entity = Some(entity);
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::BankOpen)
                            .ok();
                    }
                }
            }
            BankEvent::Show => {
                ui_state.bank_open = true;

                if !ui_state_windows.inventory_open {
                    ui_state_windows.inventory_open = true;
                }
            }
        }
    }

    if !ui_state.bank_open {
        return;
    }

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    if let Some(bank_position) = ui_state
        .bank_entity
        .and_then(|bank_entity| query_position.get(bank_entity).ok())
    {
        // If player has moved away from bank entity, close the dialog
        if player
            .position
            .position
            .xy()
            .distance(bank_position.position.xy())
            > 1000.0
        {
            ui_state.bank_open = false;
            ui_state.bank_entity = None;
            return;
        }
    }

    let mut response_close_button = None;

    egui::Window::new("Bank")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    radio: &mut [(IID_RADIOBOX, &mut ui_state.current_page)],
                    response: &mut [(IID_BTN_CLOSE, &mut response_close_button)],
                    label: &mut [
                        (
                            IID_BTN_TAB1,
                            &format!("{} {}", game_data.client_strings.bank_tab, 1),
                        ),
                        (
                            IID_BTN_TAB2,
                            &format!("{} {}", game_data.client_strings.bank_tab, 2),
                        ),
                        (
                            IID_BTN_TAB3,
                            &format!("{} {}", game_data.client_strings.bank_tab, 3),
                        ),
                        (IID_BTN_TAB4, game_data.client_strings.bank_tab_premium),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    ui.put(
                        egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(350.0, 20.0)),
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{}'s {}",
                                player.character_info.name, game_data.client_strings.bank_tab
                            ))
                            .color(egui::Color32::WHITE)
                            .font(egui::FontId::new(
                                14.0,
                                egui::FontFamily::Name("Ubuntu-M".into()),
                            )),
                        ),
                    );

                    let tab_index = match bindings.get_radio(IID_RADIOBOX) {
                        Some(&mut IID_BTN_TAB1) => 0,
                        Some(&mut IID_BTN_TAB2) => 1,
                        Some(&mut IID_BTN_TAB3) => 2,
                        Some(&mut IID_BTN_TAB4) => 3,
                        _ => 0,
                    };

                    for slot in 0..BANK_SLOTS_PER_PAGE {
                        let slot_index = tab_index * BANK_SLOTS_PER_PAGE + slot;
                        let slot_column = slot % BANK_SLOTS_PER_ROW;
                        let slot_row = slot / BANK_SLOTS_PER_ROW;
                        let pos = egui::pos2(
                            10.0 + slot_column as f32 * 41.0,
                            50.0 + slot_row as f32 * 41.0,
                        );

                        ui_add_bank_slot(
                            ui,
                            slot_index,
                            pos,
                            &player,
                            player_tooltip_data.as_ref(),
                            &game_data,
                            &ui_resources,
                            &mut ui_state_dnd,
                            &mut player_command_events,
                        );
                    }
                },
            );
        });

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state.bank_open = false;
    }
}
