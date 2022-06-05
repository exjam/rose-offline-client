use bevy::prelude::{Query, Res, Time};

use crate::components::Cooldowns;

pub fn cooldown_system(mut query_cooldowns: Query<&mut Cooldowns>, time: Res<Time>) {
    let delta = time.delta();

    for mut cooldowns in query_cooldowns.iter_mut() {
        if let Some((current, _)) = cooldowns.global.as_mut() {
            if delta < *current {
                *current -= delta;
            } else {
                cooldowns.global = None;
            }
        }

        for (_, cooldown) in cooldowns.skills.iter_mut() {
            if let Some((current, _)) = cooldown.as_mut() {
                if delta < *current {
                    *current -= delta;
                } else {
                    *cooldown = None;
                }
            }
        }

        for (_, cooldown) in cooldowns.skill_groups.iter_mut() {
            if let Some((current, _)) = cooldown.as_mut() {
                if delta < *current {
                    *current -= delta;
                } else {
                    *cooldown = None;
                }
            }
        }

        for (_, cooldown) in cooldowns.consumable_items.iter_mut() {
            if let Some((current, _)) = cooldown.as_mut() {
                if delta < *current {
                    *current -= delta;
                } else {
                    *cooldown = None;
                }
            }
        }
    }
}
