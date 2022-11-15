use bevy::prelude::Resource;
use std::collections::HashMap;

use rose_data::ItemType;
use rose_data_irose::encode_item_type;

use crate::scripting::lua4::Lua4Value;

pub const SV_SEX: i32 = 0;
pub const SV_BIRTH: i32 = 1;
pub const SV_CLASS: i32 = 2;
pub const SV_UNION: i32 = 3;
pub const SV_RANK: i32 = 4;
pub const SV_FAME: i32 = 5;
pub const SV_STR: i32 = 6;
pub const SV_DEX: i32 = 7;
pub const SV_INT: i32 = 8;
pub const SV_CON: i32 = 9;
pub const SV_CHA: i32 = 10;
pub const SV_SEN: i32 = 11;
pub const SV_EXP: i32 = 12;
pub const SV_LEVEL: i32 = 13;
pub const SV_POINT: i32 = 14;

#[derive(Resource)]
pub struct LuaGameConstants {
    pub constants: HashMap<String, Lua4Value>,
}

impl Default for LuaGameConstants {
    fn default() -> Self {
        let mut constants: HashMap<String, Lua4Value> = HashMap::new();

        constants.insert("SV_SEX".to_string(), SV_SEX.into());
        constants.insert("SV_BIRTH".to_string(), SV_BIRTH.into());
        constants.insert("SV_CLASS".to_string(), SV_CLASS.into());
        constants.insert("SV_UNION".to_string(), SV_UNION.into());
        constants.insert("SV_RANK".to_string(), SV_RANK.into());
        constants.insert("SV_FAME".to_string(), SV_FAME.into());
        constants.insert("SV_STR".to_string(), SV_STR.into());
        constants.insert("SV_DEX".to_string(), SV_DEX.into());
        constants.insert("SV_INT".to_string(), SV_INT.into());
        constants.insert("SV_CON".to_string(), SV_CON.into());
        constants.insert("SV_CHA".to_string(), SV_CHA.into());
        constants.insert("SV_SEN".to_string(), SV_SEN.into());
        constants.insert("SV_EXP".to_string(), SV_EXP.into());
        constants.insert("SV_LEVEL".to_string(), SV_LEVEL.into());
        constants.insert("SV_POINT".to_string(), SV_POINT.into());

        constants.insert(
            "ITEM_TYPE_FACE_ITEM".to_string(),
            encode_item_type(ItemType::Face).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_HELMET".to_string(),
            encode_item_type(ItemType::Head).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_ARMOR".to_string(),
            encode_item_type(ItemType::Body).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_GAUNTLET".to_string(),
            encode_item_type(ItemType::Hands).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_BOOTS".to_string(),
            encode_item_type(ItemType::Feet).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_KNAPSACK".to_string(),
            encode_item_type(ItemType::Back).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_JEWEL".to_string(),
            encode_item_type(ItemType::Jewellery).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_WEAPON".to_string(),
            encode_item_type(ItemType::Weapon).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_SUBWPN".to_string(),
            encode_item_type(ItemType::SubWeapon).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_USE".to_string(),
            encode_item_type(ItemType::Consumable).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_ETC".to_string(),
            encode_item_type(ItemType::Gem).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_GEM".to_string(),
            encode_item_type(ItemType::Gem).unwrap().into(),
        );
        constants.insert(
            "ITEM_TYPE_NATURAL".to_string(),
            encode_item_type(ItemType::Material).unwrap().into(),
        );

        Self { constants }
    }
}
