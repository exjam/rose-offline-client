use bevy::prelude::Entity;

use rose_game_common::components::{ItemSlot, SkillSlot};

use crate::components::Position;

#[derive(Clone)]
pub enum PlayerCommandEvent {
    UseSkill(SkillSlot),
    UseItem(ItemSlot),
    UseHotbar(usize, usize),
    Attack(Entity),
    Move(Position, Option<Entity>),
}
