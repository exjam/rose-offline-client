use bevy::prelude::States;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    GameLogin,
    GameCharacterSelect,
    Game,
    ModelViewer,
    ZoneViewer,
}
