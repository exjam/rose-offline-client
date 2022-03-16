use bevy::prelude::{Component, Entity};
use enum_map::{Enum, EnumMap};

use rose_data::NpcId;
use rose_game_common::components::CharacterGender;

#[derive(Component)]
pub struct CharacterModel {
    pub gender: CharacterGender,
    pub model_parts: EnumMap<CharacterModelPart, (usize, Vec<Entity>)>,
}

#[derive(Component)]
pub struct NpcModel {
    pub npc_id: NpcId,
    pub model_parts: Vec<Entity>,
}

#[derive(Component)]
pub struct ModelSkeleton {
    pub bones: Vec<Entity>,
    pub dummy_bone_offset: usize,
}

#[derive(Component, Default)]
pub struct DebugModelSkeleton;

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
