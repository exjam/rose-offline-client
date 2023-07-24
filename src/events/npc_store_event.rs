use bevy::prelude::Event;

use rose_game_common::messages::ClientEntityId;

#[derive(Event)]
pub enum NpcStoreEvent {
    OpenClientEntityStore(ClientEntityId),
    RemoveFromBuyList(usize),
    RemoveFromSellList(usize),
    AddToBuyList {
        store_tab_index: usize,
        store_tab_slot: usize,
        quantity: usize,
    },
}
