use std::time::Duration;

use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Default, Reflect, FromReflect)]
pub struct PassiveRecoveryTime {
    pub time: Duration,
}
