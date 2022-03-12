use bevy::prelude::{Component, Entity};
use enum_map::{Enum, EnumMap};

use rose_game_common::components::CharacterGender;

#[derive(Component)]
pub struct CharacterModel {
    pub gender: CharacterGender,
    pub skeleton: CharacterModelSkeleton,
    pub model_parts: EnumMap<CharacterModelPart, (usize, Vec<Entity>)>,
}

pub struct CharacterModelSkeleton {
    pub bones: Vec<Entity>,
    pub dummy_bone_offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum CharacterModelPart {
    CharacterFace,
    CharacterHair,
    Head,
    FaceItem,
    Body,
    Hands,
    Feet,
    Back,
    Weapon,
    SubWeapon,
}
