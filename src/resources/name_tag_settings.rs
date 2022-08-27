use enum_map::{enum_map, EnumMap};

use crate::components::NameTagType;

pub struct NameTagSettings {
    pub show_all: EnumMap<NameTagType, bool>,
    pub font_size: EnumMap<NameTagType, f32>,
}

impl Default for NameTagSettings {
    fn default() -> Self {
        Self {
            show_all: enum_map! {
                NameTagType::Character => true,
                NameTagType::Npc => true,
                NameTagType::Monster => false,
            },
            font_size: enum_map! {
                NameTagType::Character => 14.0,
                NameTagType::Npc => 14.0,
                NameTagType::Monster => 14.0,
            },
        }
    }
}
