use rose_game_common::messages::server::{CreateCharacterError, CreateCharacterResponse};

pub enum WorldConnectionEvent {
    CreateCharacterResponse(Result<CreateCharacterResponse, CreateCharacterError>),
}
