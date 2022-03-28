use bevy::prelude::{EventWriter, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};

use rose_data::{AmmoIndex, EquipmentIndex, Item, VehiclePartIndex};
use rose_game_common::components::{
    Equipment, Inventory, InventoryPageType, ItemSlot, INVENTORY_PAGE_SIZE,
};

use crate::{
    components::PlayerCharacter,
    events::ChatboxEvent,
    resources::{GameData, Icons},
    ui::{DragAndDropId, DragAndDropSlot, UiStateDragAndDrop},
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
        Some(ItemSlot::Equipment(EquipmentIndex::WeaponLeft)),
        Some(ItemSlot::Equipment(EquipmentIndex::Body)),
        Some(ItemSlot::Equipment(EquipmentIndex::WeaponRight)),
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

fn ui_add_inventory_slot(
    ui: &mut egui::Ui,
    inventory_slot: ItemSlot,
    equipment: &Equipment,
    inventory: &Inventory,
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

    let item = match inventory_slot {
        ItemSlot::Inventory(_, _) => inventory.get_item(inventory_slot).cloned(),
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
    };

    let item_data = item
        .as_ref()
        .and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
    let contents =
        item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize));
    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::Inventory(inventory_slot),
        contents,
        drag_accepts,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    let mut equip_inventory_slot = None;
    let mut unequip_inventory_slot = None;
    let mut use_inventory_slot = None;
    let mut drop_inventory_slot = None;
    let mut swap_inventory_slots = None;

    if response.double_clicked() {
        match inventory_slot {
            ItemSlot::Inventory(InventoryPageType::Equipment, _)
            | ItemSlot::Inventory(InventoryPageType::Vehicles, _)
            | ItemSlot::Inventory(InventoryPageType::Materials, _) => {
                equip_inventory_slot = Some(inventory_slot);
            }
            ItemSlot::Inventory(InventoryPageType::Consumables, _) => {
                use_inventory_slot = Some(inventory_slot);
            }
            ItemSlot::Equipment(_) | ItemSlot::Ammo(_) | ItemSlot::Vehicle(_) => {
                unequip_inventory_slot = Some(inventory_slot);
            }
        }
    }

    if let (Some(item), Some(item_data)) = (item, item_data) {
        let response = response.context_menu(|ui| {
            if matches!(
                inventory_slot,
                ItemSlot::Inventory(InventoryPageType::Equipment, _)
                    | ItemSlot::Inventory(InventoryPageType::Vehicles, _)
                    | ItemSlot::Inventory(InventoryPageType::Materials, _)
            ) && ui.button("Equip").clicked()
            {
                equip_inventory_slot = Some(inventory_slot);
            }

            if matches!(inventory_slot, ItemSlot::Equipment(_)) && ui.button("Unequip").clicked() {
                unequip_inventory_slot = Some(inventory_slot);
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

        response.on_hover_text(format!(
            "{}\nItem Type: {:?} Item ID: {}",
            item_data.name,
            item.get_item_type(),
            item.get_item_number()
        ));
    }

    if let Some(DragAndDropId::Inventory(dropped_inventory_slot)) = dropped_item {
        match inventory_slot {
            ItemSlot::Inventory(_, _) => match dropped_inventory_slot {
                ItemSlot::Inventory(_, _) => {
                    swap_inventory_slots = Some((inventory_slot, dropped_inventory_slot))
                }
                ItemSlot::Equipment(_) | ItemSlot::Ammo(_) | ItemSlot::Vehicle(_) => {
                    unequip_inventory_slot = Some(inventory_slot);
                }
            },
            ItemSlot::Equipment(_) => {
                if matches!(
                    dropped_inventory_slot,
                    ItemSlot::Inventory(InventoryPageType::Equipment, _)
                ) {
                    equip_inventory_slot = Some(dropped_inventory_slot);
                }
            }
            ItemSlot::Ammo(_) => {
                if matches!(
                    dropped_inventory_slot,
                    ItemSlot::Inventory(InventoryPageType::Materials, _)
                ) {
                    equip_inventory_slot = Some(dropped_inventory_slot);
                }
            }
            ItemSlot::Vehicle(_) => {
                if matches!(
                    dropped_inventory_slot,
                    ItemSlot::Inventory(InventoryPageType::Vehicles, _)
                ) {
                    equip_inventory_slot = Some(dropped_inventory_slot);
                }
            }
        }
    }

    if let Some(equip_inventory_slot) = equip_inventory_slot {
        chatbox_events.send(ChatboxEvent::System(format!(
            "TODO: Equip item {:?}",
            equip_inventory_slot
        )));
    }

    if let Some(unequip_inventory_slot) = unequip_inventory_slot {
        chatbox_events.send(ChatboxEvent::System(format!(
            "TODO: Unequip item {:?}",
            unequip_inventory_slot
        )));
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
    mut ui_state_inventory: ResMut<UiStateInventory>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    query_player: Query<(&Equipment, &Inventory), With<PlayerCharacter>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
) {
    let (player_equipment, player_inventory) = query_player.single();

    egui::Window::new("Inventory")
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
