use bevy::prelude::{EventWriter, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};

use rose_data::{AmmoIndex, EquipmentIndex, Item, ItemClass, ItemType, VehiclePartIndex};
use rose_game_common::{
    components::{Equipment, Inventory, InventoryPageType, ItemSlot, INVENTORY_PAGE_SIZE},
    messages::client::{ChangeEquipment, ClientMessage},
};

use crate::{
    components::PlayerCharacter,
    events::ChatboxEvent,
    resources::{GameConnection, GameData, Icons},
    ui::{ui_add_item_tooltip, DragAndDropId, DragAndDropSlot, UiStateDragAndDrop, UiStateWindows},
};

#[derive(Copy, Clone, PartialEq, Debug)]
enum EquipmentPageType {
    Equipment,
    Vehicle,
}

pub struct UiStateInventory {
    equipment_page: EquipmentPageType,
    current_page: InventoryPageType,
    item_slot_map: EnumMap<InventoryPageType, Vec<ItemSlot>>,
}

impl Default for UiStateInventory {
    fn default() -> Self {
        Self {
            equipment_page: EquipmentPageType::Equipment,
            current_page: InventoryPageType::Equipment,
            item_slot_map: enum_map! {
                page_type => (0..INVENTORY_PAGE_SIZE)
                .map(|index| ItemSlot::Inventory(page_type, index))
                .collect(),
            },
        }
    }
}

const EQUIPMENT_GRID_SLOTS: [[std::option::Option<rose_game_common::components::ItemSlot>; 4]; 4] = [
    [
        Some(ItemSlot::Equipment(EquipmentIndex::Face)),
        Some(ItemSlot::Equipment(EquipmentIndex::Head)),
        Some(ItemSlot::Equipment(EquipmentIndex::Back)),
        Some(ItemSlot::Ammo(AmmoIndex::Arrow)),
    ],
    [
        Some(ItemSlot::Equipment(EquipmentIndex::WeaponRight)),
        Some(ItemSlot::Equipment(EquipmentIndex::Body)),
        Some(ItemSlot::Equipment(EquipmentIndex::WeaponLeft)),
        Some(ItemSlot::Ammo(AmmoIndex::Bullet)),
    ],
    [
        Some(ItemSlot::Equipment(EquipmentIndex::Hands)),
        Some(ItemSlot::Equipment(EquipmentIndex::Feet)),
        None,
        Some(ItemSlot::Ammo(AmmoIndex::Throw)),
    ],
    [
        Some(ItemSlot::Equipment(EquipmentIndex::Ring)),
        Some(ItemSlot::Equipment(EquipmentIndex::Necklace)),
        Some(ItemSlot::Equipment(EquipmentIndex::Earring)),
        None,
    ],
];

const VEHICLE_GRID_SLOTS: [[std::option::Option<rose_game_common::components::ItemSlot>; 4]; 4] = [
    [
        Some(ItemSlot::Vehicle(VehiclePartIndex::Body)),
        None,
        None,
        None,
    ],
    [
        Some(ItemSlot::Vehicle(VehiclePartIndex::Engine)),
        None,
        None,
        None,
    ],
    [
        Some(ItemSlot::Vehicle(VehiclePartIndex::Leg)),
        None,
        None,
        None,
    ],
    [
        Some(ItemSlot::Vehicle(VehiclePartIndex::Arms)),
        None,
        None,
        None,
    ],
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
    equipment: &Equipment,
    inventory: &Inventory,
    game_connection: Option<&Res<GameConnection>>,
    game_data: &GameData,
    icons: &Icons,
    ui_state_inventory: &mut UiStateInventory,
    ui_state_dnd: &mut UiStateDragAndDrop,
    chatbox_events: &mut EventWriter<ChatboxEvent>,
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

    let item = (equipment, inventory).get_item(inventory_slot);
    let item_data = item
        .as_ref()
        .and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
    let contents =
        item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize));
    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::Inventory(inventory_slot),
        contents,
        match item.as_ref() {
            Some(Item::Stackable(stackable_item)) => Some(stackable_item.quantity as usize),
            _ => None,
        },
        None,
        drag_accepts,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

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
        if let Some(item) = inventory.get_item(equip_inventory_slot) {
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
                ItemType::Weapon => Some(EquipmentIndex::WeaponRight),
                ItemType::SubWeapon => Some(EquipmentIndex::WeaponLeft),
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
        if let Some(item) = inventory.get_item(equip_inventory_slot) {
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
                        .send(ClientMessage::ChangeAmmo(ammo_index, None))
                        .ok();
                }
            }
        }
    }

    if let Some(equip_inventory_slot) = equip_vehicle_inventory_slot {
        if let Some(item) = inventory.get_item(equip_inventory_slot) {
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
                        .send(ClientMessage::ChangeVehiclePart(vehicle_part_index, None))
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
        chatbox_events.send(ChatboxEvent::System(format!(
            "TODO: Use item {:?}",
            use_inventory_slot
        )));
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
            let inventory_map = &mut ui_state_inventory.item_slot_map[page_a];
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

pub fn ui_inventory_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_inventory: Local<UiStateInventory>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<(&Equipment, &Inventory), With<PlayerCharacter>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
) {
    let (player_equipment, player_inventory) = query_player.single();

    egui::Window::new("Inventory")
        .id(ui_state_windows.inventory_window_id)
        .open(&mut ui_state_windows.inventory_open)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_inventory.equipment_page,
                    EquipmentPageType::Equipment,
                    "Equipment",
                );
                ui.selectable_value(
                    &mut ui_state_inventory.equipment_page,
                    EquipmentPageType::Vehicle,
                    "Vehicle",
                );
            });

            let equipment_grid_slots = match ui_state_inventory.equipment_page {
                EquipmentPageType::Equipment => &EQUIPMENT_GRID_SLOTS,
                EquipmentPageType::Vehicle => &VEHICLE_GRID_SLOTS,
            };

            egui::Grid::new("inventory_equipment_grid")
                .num_columns(4)
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    for row in equipment_grid_slots.iter() {
                        for item_slot in row.iter() {
                            if let &Some(item_slot) = item_slot {
                                ui_add_inventory_slot(
                                    ui,
                                    item_slot,
                                    player_equipment,
                                    player_inventory,
                                    game_connection.as_ref(),
                                    &game_data,
                                    &icons,
                                    &mut ui_state_inventory,
                                    &mut ui_state_dnd,
                                    &mut chatbox_events,
                                );
                            } else {
                                ui.label("");
                            }
                        }

                        ui.end_row();
                    }
                });

            let current_page = match ui_state_inventory.equipment_page {
                EquipmentPageType::Equipment => {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut ui_state_inventory.current_page,
                            InventoryPageType::Equipment,
                            "Equipment",
                        );
                        ui.selectable_value(
                            &mut ui_state_inventory.current_page,
                            InventoryPageType::Consumables,
                            "Consumables",
                        );
                        ui.selectable_value(
                            &mut ui_state_inventory.current_page,
                            InventoryPageType::Materials,
                            "ETC",
                        );
                    });
                    ui_state_inventory.current_page
                }
                EquipmentPageType::Vehicle => {
                    let mut vehicle_page = InventoryPageType::Vehicles;
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut vehicle_page,
                            InventoryPageType::Vehicles,
                            "Vehicle",
                        );
                    });
                    vehicle_page
                }
            };

            egui::Grid::new("inventory_items_grid")
                .num_columns(5)
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    for row in 0..6 {
                        for column in 0..5 {
                            let inventory_slot =
                                ui_state_inventory.item_slot_map[current_page][column + row * 5];
                            ui_add_inventory_slot(
                                ui,
                                inventory_slot,
                                player_equipment,
                                player_inventory,
                                game_connection.as_ref(),
                                &game_data,
                                &icons,
                                &mut ui_state_inventory,
                                &mut ui_state_dnd,
                                &mut chatbox_events,
                            );
                        }

                        ui.end_row();
                    }
                });

            ui.label(format!("Zuly: {}", player_inventory.money.0));
        });
}
