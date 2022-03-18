use bevy::prelude::Component;

pub use rose_game_common::messages::ClientEntityId;

#[derive(Component)]
pub struct ClientEntity {
    pub id: ClientEntityId,
}

impl ClientEntity {
    pub fn new(id: ClientEntityId) -> Self {
        Self { id }
    }
}
