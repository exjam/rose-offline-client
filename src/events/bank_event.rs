use bevy::prelude::Event;

use rose_game_common::messages::ClientEntityId;

#[derive(Event)]
pub enum BankEvent {
    OpenBankFromClientEntity { client_entity_id: ClientEntityId },
    Show,
}
