use bevy::prelude::Component;
use enum_map::Enum;

#[derive(Component, Enum, Copy, Clone, Debug)]
pub enum SoundCategory {
    BackgroundMusic,
    PlayerFootstep,
    OtherFootstep,
    NpcSounds,
}
