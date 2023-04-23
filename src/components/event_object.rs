use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct EventObject {
    pub quest_trigger_name: String,
    pub script_function_name: String,
    pub last_collision: f64,
}

impl EventObject {
    pub fn new(quest_trigger_name: String, script_function_name: String) -> Self {
        Self {
            quest_trigger_name,
            script_function_name,
            last_collision: 0.0,
        }
    }
}
