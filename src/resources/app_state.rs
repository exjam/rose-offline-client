#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    GameLogin,
    GameCharacterSelect,
    ModelViewer,
    ZoneViewer,
}
