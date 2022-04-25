use rose_game_common::messages::ClientEntityId;

pub enum NpcStoreEvent {
    OpenClientEntityStore(ClientEntityId),
}
