use std::{any::Any, cmp::Ordering, sync::Arc};

use num_traits::ToPrimitive;

use crate::scripting::lua4::Lua4Function;

#[derive(Clone)]
pub enum Lua4Value {
    Nil,
    UserData(Arc<dyn Any + Send + Sync>),
    Number(f64),
    String(String),
    Table,
    Closure(Arc<Lua4Function>, Vec<Lua4Value>),
    RustClosure(String),
}

impl Lua4Value {
    pub fn to_user_type<T: Any>(&self) -> Result<&T, LuaValueConversionError> {
        if let Lua4Value::UserData(user_data) = self {
            user_data
                .downcast_ref::<T>()
                .ok_or(LuaValueConversionError::InvalidType)
        } else {
            Err(LuaValueConversionError::InvalidType)
        }
    }

    pub fn to_f32(&self) -> Result<f32, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_f64(&self) -> Result<f64, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_i32(&self) -> Result<i32, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_i64(&self) -> Result<i64, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_usize(&self) -> Result<usize, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_string(&self) -> Result<String, LuaValueConversionError> {
        self.try_into()
    }
}

impl PartialEq for Lua4Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Lua4Value::Nil => matches!(other, Lua4Value::Nil),
            Lua4Value::Number(value) => {
                if let Lua4Value::Number(other) = other {
                    value == other
                } else {
                    false
                }
            }
            Lua4Value::String(value) => {
                if let Lua4Value::String(other) = other {
                    value == other
                } else {
                    false
                }
            }
            Lua4Value::Table => todo!("Implement LuaValue::Table"),
            Lua4Value::UserData(_) => false,
            Lua4Value::Closure(_, _) => false,
            Lua4Value::RustClosure(_) => false,
        }
    }
}

impl Eq for Lua4Value {}

impl PartialOrd for Lua4Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Lua4Value::Number(value) => {
                if let Lua4Value::Number(other) = other {
                    value.partial_cmp(other)
                } else {
                    None
                }
            }
            Lua4Value::String(value) => {
                if let Lua4Value::String(other) = other {
                    value.partial_cmp(other)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum LuaValueConversionError {
    InvalidType,
}

impl From<bool> for Lua4Value {
    fn from(value: bool) -> Self {
        Lua4Value::Number(if value { 1.0 } else { 0.0 })
    }
}

impl From<i32> for Lua4Value {
    fn from(value: i32) -> Self {
        Lua4Value::Number(value as f64)
    }
}

impl From<usize> for Lua4Value {
    fn from(value: usize) -> Self {
        Lua4Value::Number(value as f64)
    }
}

impl From<f32> for Lua4Value {
    fn from(value: f32) -> Self {
        Lua4Value::Number(value as f64)
    }
}

impl From<f64> for Lua4Value {
    fn from(value: f64) -> Self {
        Lua4Value::Number(value)
    }
}

impl From<String> for Lua4Value {
    fn from(value: String) -> Self {
        Lua4Value::String(value)
    }
}

impl TryFrom<&Lua4Value> for f32 {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => Ok(*number as f32),
            Lua4Value::String(string) => string
                .parse::<f64>()
                .map_err(|_| LuaValueConversionError::InvalidType)
                .map(|value| value as f32),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}

impl TryFrom<&Lua4Value> for f64 {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => Ok(*number),
            Lua4Value::String(string) => string
                .parse::<f64>()
                .map_err(|_| LuaValueConversionError::InvalidType),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}

impl TryFrom<&Lua4Value> for i32 {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => {
                number.to_i32().ok_or(LuaValueConversionError::InvalidType)
            }
            Lua4Value::String(string) => string
                .parse::<f64>()
                .map_err(|_| LuaValueConversionError::InvalidType)
                .map(|value| value as i32),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}

impl TryFrom<&Lua4Value> for i64 {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => {
                number.to_i64().ok_or(LuaValueConversionError::InvalidType)
            }
            Lua4Value::String(string) => string
                .parse::<f64>()
                .map_err(|_| LuaValueConversionError::InvalidType)
                .map(|value| value as i64),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}

impl TryFrom<&Lua4Value> for usize {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => number
                .to_usize()
                .ok_or(LuaValueConversionError::InvalidType),
            Lua4Value::String(string) => string
                .parse::<usize>()
                .map_err(|_| LuaValueConversionError::InvalidType),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}

impl TryFrom<&Lua4Value> for String {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => Ok(format!("{}", *number)),
            Lua4Value::String(string) => Ok(string.clone()),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}
