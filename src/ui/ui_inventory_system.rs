use bevy::prelude::{Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};

use rose_data::{AmmoIndex, EquipmentIndex, VehiclePartIndex};
use rose_game_common::components::{
    Equipment, Inventory, InventoryPageType, ItemSlot, INVENTORY_PAGE_SIZE,
};

use crate::{
    components::PlayerCharacter,
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

pub fn ui_inventory_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_inventory: ResMut<UiStateInventory>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    query_player: Query<(&Equipment, &Inventory), With<PlayerCharacter>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
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
                                ui.add(DragAndDropSlot::new(
                                    DragAndDropId::Inventory(item_slot),
                                    &game_data,
                                    &icons,
                                    player_equipment,
                                    player_inventory,
                                    &mut ui_state_dnd,
                                    [40.0, 40.0],
                                ));
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
                    let inventory_map = &ui_state_inventory.item_slot_map[current_page];

                    for row in 0..6 {
                        for column in 0..5 {
                            let slot = DragAndDropSlot::new(
                                DragAndDropId::Inventory(inventory_map[column + row * 5]),
                                &game_data,
                                &icons,
                                player_equipment,
                                player_inventory,
                                &mut ui_state_dnd,
                                [40.0, 40.0],
                            );
                            ui.add(slot);
                        }

                        ui.end_row();
                    }

                    if let Some(DragAndDropId::Inventory(source_item_slot)) =
                        ui_state_dnd.source.as_ref()
                    {
                        if let Some(DragAndDropId::Inventory(destination_item_slot)) =
                            ui_state_dnd.destination.as_ref()
                        {
                            if ui.input().pointer.any_released()
                                && !ui.input().pointer.button_down(egui::PointerButton::Primary)
                            {
                                if matches!(source_item_slot, ItemSlot::Equipment(_))
                                    && matches!(destination_item_slot, ItemSlot::Inventory(_, _))
                                {
                                    // TODO: Unequip item
                                }

                                if matches!(source_item_slot, ItemSlot::Inventory(_, _))
                                    && matches!(destination_item_slot, ItemSlot::Equipment(_))
                                {
                                    // TODO: Equip item
                                }

                                // Move item within inventory
                                if matches!(source_item_slot, ItemSlot::Inventory(_, _))
                                    && matches!(destination_item_slot, ItemSlot::Inventory(_, _))
                                {
                                    let inventory_map =
                                        &mut ui_state_inventory.item_slot_map[current_page];
                                    let source_index = inventory_map
                                        .iter()
                                        .position(|slot| slot == source_item_slot);
                                    let destination_index = inventory_map
                                        .iter()
                                        .position(|slot| slot == destination_item_slot);
                                    if let (Some(source_index), Some(destination_index)) =
                                        (source_index, destination_index)
                                    {
                                        inventory_map.swap(source_index, destination_index);
                                    }
                                }

                                ui_state_dnd.source = None;
                                ui_state_dnd.destination = None;
                            }
                        }
                    }
                });

            ui.label(format!("Zuly: {}", player_inventory.money.0));
        });
}
