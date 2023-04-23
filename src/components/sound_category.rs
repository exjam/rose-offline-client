use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};
use enum_map::Enum;

#[derive(Component, Enum, Copy, Clone, Debug, Reflect, FromReflect)]
pub enum SoundCategory {
    BackgroundMusic,
    PlayerFootstep,
    OtherFootstep,
    PlayerCombat,
    OtherCombat,
    NpcSounds,
    Ui,
}
