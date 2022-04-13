use bevy::prelude::{Component, Entity};

use enum_map::EnumMap;
use rose_data::{StatusEffectId, StatusEffectType};

#[derive(Component, Default)]
pub struct VisibleStatusEffects {
    pub effects: EnumMap<StatusEffectType, Option<(StatusEffectId, Entity)>>,
}
