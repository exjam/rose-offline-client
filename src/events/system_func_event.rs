use bevy::prelude::Event;

use crate::scripting::lua4::Lua4Value;

#[derive(Event, Clone)]
pub enum SystemFuncEvent {
    CallFunction(String, Vec<Lua4Value>),
}
