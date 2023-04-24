use std::{collections::HashMap, time::Duration};

use bevy::prelude::Component;
use enum_map::{Enum, EnumMap};

use rose_data::{
    ItemClass, ItemReference, ItemType, SkillCooldownGroup, SkillId, StatusEffectType,
};

use crate::resources::GameData;

#[derive(Copy, Clone, Debug, Enum)]
pub enum ConsumableCooldownGroup {
    HealthRecovery,
    ManaRecovery,
    MagicItem,
    Others,
}

impl ConsumableCooldownGroup {
    pub fn from_item(item: &ItemReference, game_data: &GameData) -> Option<Self> {
        if item.item_type != ItemType::Consumable {
            return None;
        }

        let consumable_item_data = game_data.items.get_consumable_item(item.item_number);
        if consumable_item_data.is_none() {
            return Some(ConsumableCooldownGroup::Others);
        }
        let consumable_item_data = consumable_item_data.unwrap();

        if matches!(consumable_item_data.item_data.class, ItemClass::MagicItem) {
            Some(ConsumableCooldownGroup::MagicItem)
        } else if let Some(status_effect) =
            consumable_item_data
                .apply_status_effect
                .and_then(|(status_effect_id, _)| {
                    game_data.status_effects.get_status_effect(status_effect_id)
                })
        {
            match status_effect.status_effect_type {
                StatusEffectType::IncreaseHp => Some(ConsumableCooldownGroup::HealthRecovery),

                StatusEffectType::IncreaseMp => Some(ConsumableCooldownGroup::ManaRecovery),

                _ => Some(ConsumableCooldownGroup::Others),
            }
        } else {
            Some(ConsumableCooldownGroup::Others)
        }
    }
}

#[derive(Default, Component)]
pub struct Cooldowns {
    pub global: Option<(Duration, Duration)>,
    pub skills: HashMap<u16, Option<(Duration, Duration)>>,
    pub skill_groups: HashMap<usize, Option<(Duration, Duration)>>,
    pub consumable_items: EnumMap<ConsumableCooldownGroup, Option<(Duration, Duration)>>,
}

impl Cooldowns {
    pub fn has_global_cooldown(&self) -> bool {
        self.global.is_some()
    }

    pub fn get_global_cooldown_percent(&self) -> Option<f32> {
        if let Some((global_current, global_total)) = self.global.as_ref() {
            return Some(global_current.as_secs_f32() / global_total.as_secs_f32());
        }

        None
    }

    fn get_cooldown(&self, cooldown: Option<&(Duration, Duration)>) -> Option<f32> {
        let global_cooldown = self.get_global_cooldown_percent();

        if let Some(cooldown_percent) =
            cooldown.map(|(current, total)| current.as_secs_f32() / total.as_secs_f32())
        {
            if let Some(global_cooldown) = global_cooldown {
                Some(global_cooldown.max(cooldown_percent))
            } else {
                Some(cooldown_percent)
            }
        } else {
            global_cooldown
        }
    }

    pub fn get_consumable_cooldown_percent(&self, group: ConsumableCooldownGroup) -> Option<f32> {
        self.get_cooldown(self.consumable_items[group].as_ref())
    }

    pub fn get_skill_cooldown_percent(&self, skill_id: SkillId) -> Option<f32> {
        self.get_cooldown(self.skills.get(&skill_id.get()).and_then(|x| x.as_ref()))
    }

    pub fn get_skill_group_cooldown_percent(&self, group: SkillCooldownGroup) -> Option<f32> {
        self.get_cooldown(self.skill_groups.get(&group.get()).and_then(|x| x.as_ref()))
    }

    pub fn set_global_cooldown(&mut self, duration: Duration) {
        self.global = Some((duration, duration));
    }

    pub fn set_consumable_cooldown(&mut self, group: ConsumableCooldownGroup, cooldown: Duration) {
        self.consumable_items[group] = Some((cooldown, cooldown));
    }
}
