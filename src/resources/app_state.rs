use bevy::prelude::Resource;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Resource)]
pub enum AppState {
    GameLogin,
    GameCharacterSelect,
    Game,
    ModelViewer,
    ZoneViewer,
}
