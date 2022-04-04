use rose_game_common::messages::ClientEntityId;

pub enum ClientEntityEvent {
    LevelUp(ClientEntityId, u32),
}
