use bevy::prelude::{Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};

use rose_game_common::components::{
    Equipment, Inventory, InventoryPageType, ItemSlot, INVENTORY_PAGE_SIZE,
};

use crate::{
    components::PlayerCharacter,
    resources::{GameData, Icons},
    ui::{DragAndDropId, DragAndDropSlot, UiStateDragAndDrop},
};

pub struct UiStateInventory {
    current_page: InventoryPageType,
    item_slot_map: EnumMap<InventoryPageType, Vec<ItemSlot>>,
}

impl Default for UiStateInventory {
    fn default() -> Self {
        Self {
            current_page: InventoryPageType::Equipment,
            item_slot_map: enum_map! {
                page_type => (0..INVENTORY_PAGE_SIZE)
                .map(|index| ItemSlot::Inventory(page_type, index))
                .collect(),
            },
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
) {
    let (_player_equipment, player_inventory) = query_player.single();

    egui::Window::new("Inventory")
        .vscroll(true)
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_inventory.current_page,
                    InventoryPageType::Equipment,
                    "Equipment",
                );
                ui.selectable_value(
                    &mut ui_state_inventory.current_page,
                    InventoryPageType::Consumables,
                    "Consumeables",
                );
                ui.selectable_value(
                    &mut ui_state_inventory.current_page,
                    InventoryPageType::Materials,
                    "ETC",
                );
            });

            egui::Grid::new("inventory_items_grid")
                .num_columns(5)
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    let inventory_map =
                        &ui_state_inventory.item_slot_map[ui_state_inventory.current_page];

                    for row in 0..6 {
                        for column in 0..5 {
                            let slot = DragAndDropSlot::new(
                                DragAndDropId::Inventory(inventory_map[column + row * 5]),
                                &game_data,
                                &icons,
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
                                let current_page = ui_state_inventory.current_page;
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

                                ui_state_dnd.source = None;
                                ui_state_dnd.destination = None;
                            }
                        }
                    }
                });
        });
}
