use bevy::{prelude::Component, reflect::Reflect};
use enum_map::Enum;

#[derive(Component, Enum, Copy, Clone, Debug, Reflect)]
pub enum SoundCategory {
    BackgroundMusic,
    PlayerFootstep,
    OtherFootstep,
    PlayerCombat,
    OtherCombat,
    NpcSounds,
    Ui,
}
