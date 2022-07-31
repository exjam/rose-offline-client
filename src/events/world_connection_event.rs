use rose_game_common::messages::server::{
    CreateCharacterError, CreateCharacterResponse, DeleteCharacterError, DeleteCharacterResponse,
};

pub enum WorldConnectionEvent {
    CreateCharacterResponse(Result<CreateCharacterResponse, CreateCharacterError>),
    DeleteCharacterResponse(Result<DeleteCharacterResponse, DeleteCharacterError>),
}
