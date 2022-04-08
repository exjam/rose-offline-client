use bevy::prelude::Entity;

use rose_game_common::components::SkillSlot;

use crate::components::Position;

#[derive(Clone)]
pub enum PlayerCommandEvent {
    UseSkill(SkillSlot),
    UseHotbar(usize, usize),
    Attack(Entity),
    Move(Position, Option<Entity>),
}
