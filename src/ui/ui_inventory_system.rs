use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, EventWriter, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};

use rose_data::{
    AmmoIndex, EquipmentIndex, Item, ItemClass, ItemType, StatusEffectType, VehiclePartIndex,
};
use rose_game_common::{
    components::{Equipment, Inventory, InventoryPageType, ItemSlot, INVENTORY_PAGE_SIZE},
    messages::client::{ChangeEquipment, ClientMessage},
};

use crate::{
    components::{ConsumableCooldownGroup, Cooldowns, PlayerCharacter},
    events::{ChatboxEvent, PlayerCommandEvent},
    resources::{GameConnection, GameData, UiResources, UiSpriteSheetType},
    ui::{
        ui_add_item_tooltip,
        widgets::{DataBindings, Dialog, Widget},
        DialogInstance, DragAndDropId, DragAndDropSlot, UiStateDragAndDrop, UiStateWindows,
    },
};

const IID_BTN_CLOSE: i32 = 10;
// const IID_BTN_ICONIZE: i32 = 11;
// const IID_BTN_MONEY: i32 = 12;
const IID_TABBEDPANE_EQUIP: i32 = 20;
const IID_TAB_EQUIP_PAT: i32 = 21;
// const IID_BTN_EQUIP_PAT: i32 = 23;
const IID_TAB_EQUIP_AVATAR: i32 = 31;
// const IID_BTN_EQUIP_AVATAR: i32 = 33;
const IID_TABBEDPANE_INVEN_ITEM: i32 = 50;
const IID_TAB_INVEN_EQUIP: i32 = 51;
// const IID_BTN_INVEN_EQUIP: i32 = 53;
const IID_TAB_INVEN_USE: i32 = 61;
// const IID_BTN_INVEN_USE: i32 = 63;
const IID_TAB_INVEN_ETC: i32 = 71;
// const IID_BTN_INVEN_ETC: i32 = 73;
const IID_TABBEDPANE_INVEN_PAT: i32 = 100;
const IID_TAB_INVEN_PAT: i32 = 101;
// const IID_PANE_EQUIP: i32 = 200;
const IID_BTN_MINIMIZE: i32 = 213;
const IID_BTN_MAXIMIZE: i32 = 214;
const IID_PANE_INVEN: i32 = 300;

pub struct UiStateInventory {
    dialog_instance: DialogInstance,
    item_slot_map: EnumMap<InventoryPageType, Vec<ItemSlot>>,
    current_equipment_tab: i32,
    current_vehicle_tab: i32,
    current_inventory_tab: i32,
    minimised: bool,
}

impl Default for UiStateInventory {
    fn default() -> Self {
        Self {
            dialog_instance: DialogInstance::new("DLGITEM.XML"),
            item_slot_map: enum_map! {
                page_type => (0..INVENTORY_PAGE_SIZE)
                .map(|index| ItemSlot::Inventory(page_type, index))
                .collect(),
            },
            current_equipment_tab: IID_TAB_EQUIP_AVATAR,
            current_vehicle_tab: IID_TAB_INVEN_PAT,
            current_inventory_tab: IID_TAB_INVEN_EQUIP,
            minimised: false,
        }
    }
}

const EQUIPMENT_GRID_SLOTS: [(rose_game_common::components::ItemSlot, egui::Pos2); 14] = [
    (
        ItemSlot::Equipment(EquipmentIndex::Face),
        egui::pos2(19.0, 67.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::Head),
        egui::pos2(69.0, 67.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::Back),
        egui::pos2(119.0, 67.0),
    ),
    (ItemSlot::Ammo(AmmoIndex::Arrow), egui::pos2(169.0, 67.0)),
    (
        ItemSlot::Equipment(EquipmentIndex::Weapon),
        egui::pos2(19.0, 113.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::Body),
        egui::pos2(69.0, 113.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::SubWeapon),
        egui::pos2(119.0, 113.0),
    ),
    (ItemSlot::Ammo(AmmoIndex::Bullet), egui::pos2(169.0, 113.0)),
    (
        ItemSlot::Equipment(EquipmentIndex::Hands),
        egui::pos2(19.0, 159.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::Feet),
        egui::pos2(19.0, 159.0),
    ),
    (ItemSlot::Ammo(AmmoIndex::Throw), egui::pos2(169.0, 159.0)),
    (
        ItemSlot::Equipment(EquipmentIndex::Ring),
        egui::pos2(19.0, 205.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::Necklace),
        egui::pos2(69.0, 205.0),
    ),
    (
        ItemSlot::Equipment(EquipmentIndex::Earring),
        egui::pos2(119.0, 205.0),
    ),
];

const VEHICLE_GRID_SLOTS: [(rose_game_common::components::ItemSlot, egui::Pos2); 4] = [
    (
        ItemSlot::Vehicle(VehiclePartIndex::Body),
        egui::pos2(19.0, 68.0),
    ),
    (
        ItemSlot::Vehicle(VehiclePartIndex::Engine),
        egui::pos2(19.0, 114.0),
    ),
    (
        ItemSlot::Vehicle(VehiclePartIndex::Leg),
        egui::pos2(19.0, 160.0),
    ),
    (
        ItemSlot::Vehicle(VehiclePartIndex::Arms),
        egui::pos2(19.0, 206.0),
    ),
];

fn drag_accepts_equipment(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(ItemSlot::Inventory(InventoryPageType::Equipment, _))
            | DragAndDropId::Inventory(ItemSlot::Equipment(_))
    )
}

fn drag_accepts_consumables(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(ItemSlot::Inventory(InventoryPageType::Consumables, _))
    )
}

fn drag_accepts_materials(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(ItemSlot::Inventory(InventoryPageType::Materials, _))
            | DragAndDropId::Inventory(ItemSlot::Ammo(_))
    )
}

fn drag_accepts_vehicles(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(ItemSlot::Inventory(InventoryPageType::Vehicles, _))
            | DragAndDropId::Inventory(ItemSlot::Vehicle(_))
    )
}

pub trait GetItem {
    fn get_item(&self, item_slot: ItemSlot) -> Option<Item>;
}

impl GetItem for (&Equipment, &Inventory) {
    fn get_item(&self, item_slot: ItemSlot) -> Option<Item> {
        let equipment = self.0;
        let inventory = self.1;

        match item_slot {
            ItemSlot::Inventory(_, _) => inventory.get_item(item_slot).cloned(),
            ItemSlot::Equipment(equipment_index) => equipment
                .get_equipment_item(equipment_index)
                .cloned()
                .map(Item::Equipment),
            ItemSlot::Ammo(ammo_index) => equipment
                .get_ammo_item(ammo_index)
                .cloned()
                .map(Item::Stackable),
            ItemSlot::Vehicle(vehicle_part_index) => equipment
                .get_vehicle_item(vehicle_part_index)
                .cloned()
                .map(Item::Equipment),
        }
    }
}

fn ui_add_inventory_slot(
    ui: &mut egui::Ui,
    inventory_slot: ItemSlot,
    pos: egui::Pos2,
    player: &PlayerQueryItem,
    game_connection: Option<&Res<GameConnection>>,
    game_data: &GameData,
    ui_resources: &UiResources,
    item_slot_map: &mut EnumMap<InventoryPageType, Vec<ItemSlot>>,
    ui_state_dnd: &mut UiStateDragAndDrop,
    chatbox_events: &mut EventWriter<ChatboxEvent>,
    player_command_events: &mut EventWriter<PlayerCommandEvent>,
) {
    let drag_accepts = match inventory_slot {
        ItemSlot::Inventory(page_type, _) => match page_type {
            InventoryPageType::Equipment => drag_accepts_equipment,
            InventoryPageType::Consumables => drag_accepts_consumables,
            InventoryPageType::Materials => drag_accepts_materials,
            InventoryPageType::Vehicles => drag_accepts_vehicles,
        },
        ItemSlot::Equipment(_) => drag_accepts_equipment,
        ItemSlot::Ammo(_) => drag_accepts_materials,
        ItemSlot::Vehicle(_) => drag_accepts_vehicles,
    };

    let item = (player.equipment, player.inventory).get_item(inventory_slot);
    let item_data = item
        .as_ref()
        .and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
    let sprite = item_data.and_then(|item_data| {
        ui_resources.get_sprite_by_index(UiSpriteSheetType::Item, item_data.icon_index as usize)
    });

    let mut cooldown_percent = None;
    if let Some(item) = item.as_ref() {
        if item.get_item_type() == ItemType::Consumable {
            if let Some(consumable_item_data) =
                game_data.items.get_consumable_item(item.get_item_number())
            {
                if matches!(consumable_item_data.item_data.class, ItemClass::MagicItem) {
                    cooldown_percent = player
                        .cooldowns
                        .get_consumable_cooldown_percent(ConsumableCooldownGroup::MagicItem);
                } else if let Some(status_effect) = consumable_item_data
                    .apply_status_effect
                    .and_then(|(status_effect_id, _)| {
                        game_data.status_effects.get_status_effect(status_effect_id)
                    })
                {
                    match status_effect.status_effect_type {
                        StatusEffectType::IncreaseHp => {
                            cooldown_percent = player.cooldowns.get_consumable_cooldown_percent(
                                ConsumableCooldownGroup::HealthRecovery,
                            )
                        }
                        StatusEffectType::IncreaseMp => {
                            cooldown_percent = player.cooldowns.get_consumable_cooldown_percent(
                                ConsumableCooldownGroup::ManaRecovery,
                            )
                        }
                        _ => {
                            cooldown_percent = player
                                .cooldowns
                                .get_consumable_cooldown_percent(ConsumableCooldownGroup::Others)
                        }
                    }
                } else {
                    cooldown_percent = player
                        .cooldowns
                        .get_consumable_cooldown_percent(ConsumableCooldownGroup::Others);
                }
            }
        }
    }

    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::new(
                        DragAndDropId::Inventory(inventory_slot),
                        sprite,
                        match item.as_ref() {
                            Some(Item::Stackable(stackable_item)) => {
                                Some(stackable_item.quantity as usize)
                            }
                            _ => None,
                        },
                        cooldown_percent,
                        drag_accepts,
                        &mut ui_state_dnd.dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    let mut equip_equipment_inventory_slot = None;
    let mut equip_ammo_inventory_slot = None;
    let mut equip_vehicle_inventory_slot = None;
    let mut unequip_equipment_index = None;
    let mut unequip_ammo_index = None;
    let mut unequip_vehicle_part_index = None;
    let mut use_inventory_slot = None;
    let mut drop_inventory_slot = None;
    let mut swap_inventory_slots = None;

    if response.double_clicked() {
        match inventory_slot {
            ItemSlot::Inventory(InventoryPageType::Equipment, _) => {
                equip_equipment_inventory_slot = Some(inventory_slot);
            }
            ItemSlot::Inventory(InventoryPageType::Vehicles, _) => {
                equip_vehicle_inventory_slot = Some(inventory_slot);
            }
            ItemSlot::Inventory(InventoryPageType::Materials, _) => {
                equip_ammo_inventory_slot = Some(inventory_slot);
            }
            ItemSlot::Inventory(InventoryPageType::Consumables, _) => {
                use_inventory_slot = Some(inventory_slot);
            }
            ItemSlot::Equipment(equipment_index) => {
                unequip_equipment_index = Some(equipment_index);
            }
            ItemSlot::Ammo(ammo_index) => {
                unequip_ammo_index = Some(ammo_index);
            }
            ItemSlot::Vehicle(vehicle_part_index) => {
                unequip_vehicle_part_index = Some(vehicle_part_index);
            }
        }
    }

    if let Some(item) = item {
        let response = response.context_menu(|ui| {
            if matches!(
                inventory_slot,
                ItemSlot::Inventory(InventoryPageType::Equipment, _)
            ) && ui.button("Equip").clicked()
            {
                equip_equipment_inventory_slot = Some(inventory_slot);
            }

            if matches!(
                inventory_slot,
                    | ItemSlot::Inventory(InventoryPageType::Vehicles, _)
            ) && ui.button("Equip").clicked()
            {
                equip_vehicle_inventory_slot = Some(inventory_slot);
            }

            if matches!(
                inventory_slot,
                    | ItemSlot::Inventory(InventoryPageType::Materials, _)
            ) && ui.button("Equip").clicked()
            {
                equip_ammo_inventory_slot = Some(inventory_slot);
            }

            if let ItemSlot::Equipment(equipment_index) = inventory_slot {
                if ui.button("Unequip").clicked() {
                    unequip_equipment_index = Some(equipment_index);
                }
            }

            if matches!(
                inventory_slot,
                ItemSlot::Inventory(InventoryPageType::Consumables, _)
            ) && ui.button("Use").clicked()
            {
                use_inventory_slot = Some(inventory_slot);
            }

            if matches!(inventory_slot, ItemSlot::Inventory(_, _)) && ui.button("Drop").clicked() {
                drop_inventory_slot = Some(inventory_slot);
            }
        });

        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, &item);
        });
    }

    if let Some(DragAndDropId::Inventory(dropped_inventory_slot)) = dropped_item {
        match inventory_slot {
            ItemSlot::Inventory(_, _) => match dropped_inventory_slot {
                ItemSlot::Inventory(_, _) => {
                    swap_inventory_slots = Some((inventory_slot, dropped_inventory_slot))
                }
                ItemSlot::Equipment(equipment_index) => {
                    unequip_equipment_index = Some(equipment_index);
                }
                ItemSlot::Ammo(ammo_index) => {
                    unequip_ammo_index = Some(ammo_index);
                }
                ItemSlot::Vehicle(vehicle_part_index) => {
                    unequip_vehicle_part_index = Some(vehicle_part_index);
                }
            },
            ItemSlot::Equipment(_) => {
                if matches!(
                    dropped_inventory_slot,
                    ItemSlot::Inventory(InventoryPageType::Equipment, _)
                ) {
                    equip_equipment_inventory_slot = Some(dropped_inventory_slot);
                }
            }
            ItemSlot::Ammo(_) => {
                if matches!(
                    dropped_inventory_slot,
                    ItemSlot::Inventory(InventoryPageType::Materials, _)
                ) {
                    equip_ammo_inventory_slot = Some(dropped_inventory_slot);
                }
            }
            ItemSlot::Vehicle(_) => {
                if matches!(
                    dropped_inventory_slot,
                    ItemSlot::Inventory(InventoryPageType::Vehicles, _)
                ) {
                    equip_vehicle_inventory_slot = Some(dropped_inventory_slot);
                }
            }
        }
    }

    if let Some(equip_inventory_slot) = equip_equipment_inventory_slot {
        if let Some(item) = player.inventory.get_item(equip_inventory_slot) {
            let equipment_index = match item.get_item_type() {
                ItemType::Face => Some(EquipmentIndex::Face),
                ItemType::Head => Some(EquipmentIndex::Head),
                ItemType::Body => Some(EquipmentIndex::Body),
                ItemType::Hands => Some(EquipmentIndex::Hands),
                ItemType::Feet => Some(EquipmentIndex::Feet),
                ItemType::Back => Some(EquipmentIndex::Back),
                ItemType::Jewellery => {
                    if let Some(item_data) =
                        game_data.items.get_base_item(item.get_item_reference())
                    {
                        match item_data.class {
                            ItemClass::Ring => Some(EquipmentIndex::Ring),
                            ItemClass::Necklace => Some(EquipmentIndex::Necklace),
                            ItemClass::Earring => Some(EquipmentIndex::Earring),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                ItemType::Weapon => Some(EquipmentIndex::Weapon),
                ItemType::SubWeapon => Some(EquipmentIndex::SubWeapon),
                _ => None,
            };

            if let Some(equipment_index) = equipment_index {
                if let Some(game_connection) = game_connection {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeEquipment(ChangeEquipment {
                            equipment_index,
                            item_slot: Some(equip_inventory_slot),
                        }))
                        .ok();
                }
            }
        }
    }

    if let Some(equip_inventory_slot) = equip_ammo_inventory_slot {
        if let Some(item) = player.inventory.get_item(equip_inventory_slot) {
            let ammo_index =
                if let Some(item_data) = game_data.items.get_base_item(item.get_item_reference()) {
                    match item_data.class {
                        ItemClass::Arrow => Some(AmmoIndex::Arrow),
                        ItemClass::Bullet => Some(AmmoIndex::Bullet),
                        ItemClass::Shell => Some(AmmoIndex::Throw),
                        _ => None,
                    }
                } else {
                    None
                };

            if let Some(ammo_index) = ammo_index {
                if let Some(game_connection) = game_connection {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeAmmo(
                            ammo_index,
                            Some(equip_inventory_slot),
                        ))
                        .ok();
                }
            }
        }
    }

    if let Some(equip_inventory_slot) = equip_vehicle_inventory_slot {
        if let Some(item) = player.inventory.get_item(equip_inventory_slot) {
            let vehicle_part_index = if let Some(item_data) =
                game_data.items.get_base_item(item.get_item_reference())
            {
                match item_data.class {
                    ItemClass::CartBody | ItemClass::CastleGearBody => Some(VehiclePartIndex::Body),
                    ItemClass::CartEngine | ItemClass::CastleGearEngine => {
                        Some(VehiclePartIndex::Engine)
                    }
                    ItemClass::CartWheels | ItemClass::CastleGearLeg => Some(VehiclePartIndex::Leg),
                    ItemClass::CartAccessory | ItemClass::CastleGearWeapon => {
                        Some(VehiclePartIndex::Arms)
                    }
                    _ => None,
                }
            } else {
                None
            };

            if let Some(vehicle_part_index) = vehicle_part_index {
                if let Some(game_connection) = game_connection {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeVehiclePart(
                            vehicle_part_index,
                            Some(equip_inventory_slot),
                        ))
                        .ok();
                }
            }
        }
    }

    if let Some(unequip_ammo_index) = unequip_ammo_index {
        if let Some(game_connection) = game_connection {
            game_connection
                .client_message_tx
                .send(ClientMessage::ChangeAmmo(unequip_ammo_index, None))
                .ok();
        }
    }

    if let Some(unequip_equipment_index) = unequip_equipment_index {
        if let Some(game_connection) = game_connection {
            game_connection
                .client_message_tx
                .send(ClientMessage::ChangeEquipment(ChangeEquipment {
                    equipment_index: unequip_equipment_index,
                    item_slot: None,
                }))
                .ok();
        }
    }

    if let Some(unequip_vehicle_part_index) = unequip_vehicle_part_index {
        if let Some(game_connection) = game_connection {
            game_connection
                .client_message_tx
                .send(ClientMessage::ChangeVehiclePart(
                    unequip_vehicle_part_index,
                    None,
                ))
                .ok();
        }
    }

    if let Some(use_inventory_slot) = use_inventory_slot {
        player_command_events.send(PlayerCommandEvent::UseItem(use_inventory_slot));
    }

    if let Some(drop_inventory_slot) = drop_inventory_slot {
        chatbox_events.send(ChatboxEvent::System(format!(
            "TODO: Drop item {:?}",
            drop_inventory_slot
        )));
    }

    if let Some((ItemSlot::Inventory(page_a, slot_a), ItemSlot::Inventory(page_b, slot_b))) =
        swap_inventory_slots
    {
        if page_a == page_b {
            let inventory_map = &mut item_slot_map[page_a];
            let source_index = inventory_map
                .iter()
                .position(|slot| slot == &ItemSlot::Inventory(page_a, slot_a));
            let destination_index = inventory_map
                .iter()
                .position(|slot| slot == &ItemSlot::Inventory(page_b, slot_b));
            if let (Some(source_index), Some(destination_index)) = (source_index, destination_index)
            {
                inventory_map.swap(source_index, destination_index);
            }
        }
    }
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    equipment: &'w Equipment,
    inventory: &'w Inventory,
    cooldowns: &'w Cooldowns,
}

pub fn ui_inventory_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_inventory: Local<UiStateInventory>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    dialog_assets: Res<Assets<Dialog>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
) {
    let ui_state_inventory = &mut *ui_state_inventory;
    let dialog = if let Some(dialog) = ui_state_inventory
        .dialog_instance
        .get_mut(&dialog_assets, &ui_resources)
    {
        dialog
    } else {
        return;
    };
    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    let mut response_close_button = None;
    let mut response_minimise_button = None;
    let mut response_maximise_button = None;
    let is_equipment_tab = ui_state_inventory.current_equipment_tab == IID_TAB_EQUIP_AVATAR;
    let is_minimised = ui_state_inventory.minimised;

    egui::Window::new("Inventory")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.inventory_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    tabs: &mut [
                        (
                            IID_TABBEDPANE_EQUIP,
                            &mut ui_state_inventory.current_equipment_tab,
                        ),
                        (
                            IID_TABBEDPANE_INVEN_PAT,
                            &mut ui_state_inventory.current_vehicle_tab,
                        ),
                        (
                            IID_TABBEDPANE_INVEN_ITEM,
                            &mut ui_state_inventory.current_inventory_tab,
                        ),
                    ],
                    visible: &mut [
                        (IID_TABBEDPANE_INVEN_ITEM, is_equipment_tab),
                        (IID_TABBEDPANE_INVEN_PAT, !is_equipment_tab),
                        (IID_BTN_MINIMIZE, !is_minimised),
                        (IID_BTN_MAXIMIZE, is_minimised),
                    ],
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_MINIMIZE, &mut response_minimise_button),
                        (IID_BTN_MAXIMIZE, &mut response_maximise_button),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    let mut current_page = InventoryPageType::Equipment;

                    match bindings.get_tab(IID_TABBEDPANE_EQUIP) {
                        Some(&mut IID_TAB_EQUIP_AVATAR) => {
                            if !ui_state_inventory.minimised {
                                for (item_slot, pos) in EQUIPMENT_GRID_SLOTS.iter() {
                                    ui_add_inventory_slot(
                                        ui,
                                        *item_slot,
                                        *pos + egui::vec2(-1.0, -1.0),
                                        &player,
                                        game_connection.as_ref(),
                                        &game_data,
                                        &ui_resources,
                                        &mut ui_state_inventory.item_slot_map,
                                        &mut ui_state_dnd,
                                        &mut chatbox_events,
                                        &mut player_command_events,
                                    );
                                }
                            }

                            match bindings.get_tab(IID_TABBEDPANE_INVEN_ITEM) {
                                Some(&mut IID_TAB_INVEN_EQUIP) => {
                                    current_page = InventoryPageType::Equipment;
                                }
                                Some(&mut IID_TAB_INVEN_USE) => {
                                    current_page = InventoryPageType::Consumables;
                                }
                                Some(&mut IID_TAB_INVEN_ETC) => {
                                    current_page = InventoryPageType::Materials;
                                }
                                _ => {}
                            }
                        }
                        Some(&mut IID_TAB_EQUIP_PAT) => {
                            if !ui_state_inventory.minimised {
                                for (item_slot, pos) in VEHICLE_GRID_SLOTS.iter() {
                                    ui_add_inventory_slot(
                                        ui,
                                        *item_slot,
                                        *pos - egui::vec2(-1.0, -1.0),
                                        &player,
                                        game_connection.as_ref(),
                                        &game_data,
                                        &ui_resources,
                                        &mut ui_state_inventory.item_slot_map,
                                        &mut ui_state_dnd,
                                        &mut chatbox_events,
                                        &mut player_command_events,
                                    );
                                }
                            }

                            current_page = InventoryPageType::Vehicles;
                        }
                        _ => {}
                    }

                    let y_start = if ui_state_inventory.minimised {
                        83.0
                    } else {
                        283.0
                    };

                    for row in 0..6 {
                        for column in 0..5 {
                            let inventory_slot =
                                ui_state_inventory.item_slot_map[current_page][column + row * 5];

                            ui_add_inventory_slot(
                                ui,
                                inventory_slot,
                                egui::pos2(
                                    12.0 + column as f32 * 41.0,
                                    y_start + row as f32 * 41.0,
                                ),
                                &player,
                                game_connection.as_ref(),
                                &game_data,
                                &ui_resources,
                                &mut ui_state_inventory.item_slot_map,
                                &mut ui_state_dnd,
                                &mut chatbox_events,
                                &mut player_command_events,
                            );
                        }

                        ui.end_row();
                    }

                    ui.allocate_ui_at_rect(
                        ui.min_rect().translate(egui::vec2(
                            40.0,
                            dialog.height - 25.0 - if is_minimised { 200.0 } else { 0.0 },
                        )),
                        |ui| {
                            ui.horizontal_top(|ui| {
                                ui.add(egui::Label::new(format!("{}", player.inventory.money.0)))
                            })
                            .inner
                        },
                    );
                },
            );
        });

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.inventory_open = false;
    }

    if response_minimise_button.map_or(false, |r| r.clicked()) {
        ui_state_inventory.minimised = true;

        if let Some(Widget::Pane(pane)) = dialog.get_widget_mut(IID_PANE_INVEN) {
            pane.y = 54.0;
        }
    }

    if response_maximise_button.map_or(false, |r| r.clicked()) {
        ui_state_inventory.minimised = false;

        if let Some(Widget::Pane(pane)) = dialog.get_widget_mut(IID_PANE_INVEN) {
            pane.y = 254.0;
        }
    }
}
