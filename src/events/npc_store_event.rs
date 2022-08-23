use rose_game_common::messages::ClientEntityId;

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
