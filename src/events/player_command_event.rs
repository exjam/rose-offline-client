use bevy::prelude::Entity;

use rose_game_common::components::{HotbarSlot, ItemSlot, SkillSlot};

use crate::components::Position;

#[derive(Clone)]
pub enum PlayerCommandEvent {
    UseSkill(SkillSlot),
    UseItem(ItemSlot),
    UseHotbar(usize, usize),
    SetHotbar(usize, usize, Option<HotbarSlot>),
    Attack(Entity),
    Move(Position, Option<Entity>),
}
