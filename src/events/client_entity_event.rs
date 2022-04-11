use rose_data::ItemReference;
use rose_game_common::messages::ClientEntityId;

#[derive(Copy, Clone, Debug)]
pub enum ClientEntityEvent {
    LevelUp(ClientEntityId, u32),
    UseItem(ClientEntityId, ItemReference),
}
