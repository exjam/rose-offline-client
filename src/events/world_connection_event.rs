use bevy::prelude::Event;

use rose_game_common::{components::CharacterDeleteTime, messages::server::CreateCharacterError};

#[derive(Event)]
pub enum WorldConnectionEvent {
    CreateCharacterSuccess {
        character_slot: usize,
    },
    CreateCharacterError {
        error: CreateCharacterError,
    },
    DeleteCharacterStart {
        name: String,
        delete_time: CharacterDeleteTime,
    },
    DeleteCharacterCancel {
        name: String,
    },
    DeleteCharacterError {
        name: String,
    },
}
