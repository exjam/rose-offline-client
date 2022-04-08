use bevy::prelude::Component;
use rose_data::{SkillCooldownGroup, SkillId};
use std::{collections::HashMap, time::Duration};

#[derive(Default, Component)]
pub struct Cooldowns {
    pub global: Option<(Duration, Duration)>,
    pub skills: HashMap<u16, Option<(Duration, Duration)>>,
    pub skill_groups: HashMap<usize, Option<(Duration, Duration)>>,
}

impl Cooldowns {
    pub fn set_global_cooldown(&mut self, duration: Duration) {
        self.global = Some((duration, duration));
    }

    pub fn has_global_cooldown(&self) -> bool {
        self.global.is_some()
    }

    pub fn get_skill_cooldown_percent(&self, skill_id: SkillId) -> Option<f32> {
        if let Some((cooldown_current, cooldown_total)) =
            self.skills.get(&skill_id.get()).and_then(|x| x.as_ref())
        {
            if let Some((global_current, global_total)) = self.global.as_ref() {
                if global_current > cooldown_current {
                    return Some(global_current.as_secs_f32() / global_total.as_secs_f32());
                }
            }

            return Some(cooldown_current.as_secs_f32() / cooldown_total.as_secs_f32());
        }

        if let Some((global_current, global_total)) = self.global.as_ref() {
            return Some(global_current.as_secs_f32() / global_total.as_secs_f32());
        }

        None
    }

    pub fn get_skill_group_cooldown_percent(&self, group: SkillCooldownGroup) -> Option<f32> {
        if let Some((cooldown_current, cooldown_total)) = self
            .skill_groups
            .get(&group.0.get())
            .and_then(|x| x.as_ref())
        {
            if let Some((global_current, global_total)) = self.global.as_ref() {
                if global_current > cooldown_current {
                    return Some(global_current.as_secs_f32() / global_total.as_secs_f32());
                }
            }

            return Some(cooldown_current.as_secs_f32() / cooldown_total.as_secs_f32());
        }

        if let Some((global_current, global_total)) = self.global.as_ref() {
            return Some(global_current.as_secs_f32() / global_total.as_secs_f32());
        }

        None
    }
}
