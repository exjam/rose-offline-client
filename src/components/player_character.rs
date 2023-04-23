use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct PlayerCharacter;
