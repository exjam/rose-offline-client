use bevy::prelude::{Component, Entity, Handle, Vec3};
use enum_map::EnumMap;

use rose_data::{NpcId, NpcMotionAction};

use crate::zmo_asset_loader::ZmoAsset;

#[derive(Component)]
pub struct NpcModel {
    pub npc_id: NpcId,
    pub model_parts: Vec<Entity>,
    pub action_motions: EnumMap<NpcMotionAction, Handle<ZmoAsset>>,
    pub root_bone_position: Vec3,
}
