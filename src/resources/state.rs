#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Game,
    ModelViewer,
    ZoneViewer,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    Login,
    CharacterSelect,
    Game,
}
