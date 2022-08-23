use bevy::prelude::{EventWriter, Local, ResMut};
use bevy_egui::{egui, EguiContext};
use rose_game_common::components::ItemSlot;

use crate::{
    events::{NpcStoreEvent, PlayerCommandEvent},
    ui::DragAndDropId,
};

#[derive(Default)]
pub struct UiStateDragAndDrop {
    pub dragged_item: Option<DragAndDropId>,
}

pub fn ui_drag_and_drop_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut last_dropped_item: Local<Option<DragAndDropId>>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    mut npc_store_events: EventWriter<NpcStoreEvent>,
) {
    let ctx = egui_context.ctx_mut();

    // Handle a drag and drop which was dropped on nothing
    if let Some(last_dropped_item) = last_dropped_item.take() {
        if !ctx.is_pointer_over_area() {
            match last_dropped_item {
                DragAndDropId::Inventory(item_slot) => match item_slot {
                    ItemSlot::Inventory(_, _) => {
                        player_command_events.send(PlayerCommandEvent::DropItem(item_slot));
                    }
                    ItemSlot::Ammo(ammo_index) => {
                        player_command_events.send(PlayerCommandEvent::UnequipAmmo(ammo_index));
                    }
                    ItemSlot::Equipment(equipment_index) => {
                        player_command_events
                            .send(PlayerCommandEvent::UnequipEquipment(equipment_index));
                    }
                    ItemSlot::Vehicle(vehicle_part_index) => {
                        player_command_events
                            .send(PlayerCommandEvent::UnequipVehicle(vehicle_part_index));
                    }
                },
                DragAndDropId::Hotbar(page, slot) => {
                    player_command_events.send(PlayerCommandEvent::SetHotbar(page, slot, None));
                }
                DragAndDropId::NpcStoreBuyList(index) => {
                    npc_store_events.send(NpcStoreEvent::RemoveFromBuyList(index));
                }
                DragAndDropId::NpcStoreSellList(index) => {
                    npc_store_events.send(NpcStoreEvent::RemoveFromSellList(index));
                }
                _ => {}
            }
        }
    }

    let input = ctx.input();
    if ui_state_dnd.dragged_item.is_some()
        && input.pointer.any_released()
        && !input.pointer.button_down(egui::PointerButton::Primary)
    {
        *last_dropped_item = ui_state_dnd.dragged_item.take();
    }
}
