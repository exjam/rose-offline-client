use bevy::prelude::{Component, Entity, Handle};
use enum_map::{Enum, EnumMap};

use rose_data::CharacterMotionAction;
use rose_game_common::components::CharacterGender;

use crate::animation::ZmoAsset;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharacterModelPartIndex {
    pub id: usize,
    pub gem: usize,
    pub grade: usize,
}

#[derive(Component)]
pub struct CharacterModel {
    pub gender: CharacterGender,
    pub model_parts: EnumMap<CharacterModelPart, (CharacterModelPartIndex, Vec<Entity>)>,
    pub action_motions: EnumMap<CharacterMotionAction, Handle<ZmoAsset>>,
}
