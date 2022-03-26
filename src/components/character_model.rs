use bevy::prelude::{Component, Entity, Handle};
use enum_map::{Enum, EnumMap};

use rose_data::CharacterMotionAction;
use rose_game_common::components::CharacterGender;

use crate::zmo_asset_loader::ZmoAsset;

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

#[derive(Component)]
pub struct CharacterModel {
    pub gender: CharacterGender,
    pub model_parts: EnumMap<CharacterModelPart, (usize, Vec<Entity>)>,
    pub dummy_bone_offset: usize,
    pub action_motions: EnumMap<CharacterMotionAction, Handle<ZmoAsset>>,
}
