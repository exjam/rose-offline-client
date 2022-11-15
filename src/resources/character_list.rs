use bevy::prelude::Resource;

use rose_game_common::messages::server::CharacterListItem;

#[derive(Resource)]
pub struct CharacterList {
    pub characters: Vec<CharacterListItem>,
}
