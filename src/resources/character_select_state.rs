use bevy::prelude::Resource;

#[derive(Resource)]
pub enum CharacterSelectState {
    Entering,
    CharacterSelect(Option<usize>),
    CharacterCreate,
    CharacterCreating,
    ConnectingGameServer,
    Leaving,
    Loading,
}
