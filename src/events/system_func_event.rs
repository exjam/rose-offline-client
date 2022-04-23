use crate::scripting::lua4::Lua4Value;

#[derive(Clone)]
pub enum SystemFuncEvent {
    CallFunction(String, Vec<Lua4Value>),
}
