pub enum CharacterSelectState {
    Entering,
    CharacterSelect(Option<usize>),
    CharacterCreate,
    CharacterCreating,
    ConnectingGameServer,
    Leaving,
    Loading,
}
