use bevy::prelude::{Component, Deref, DerefMut, Entity};
use enum_map::Enum;

#[derive(Copy, Clone, Enum)]
pub enum NameTagType {
    Character,
    Monster,
    Npc,
}

#[derive(Component)]
pub struct NameTag {
    pub name_tag_type: NameTagType,
}

#[derive(Component)]
pub struct NameTagTargetMark;

#[derive(Component)]
pub struct NameTagHealthbarForeground {
    pub uv_min_x: f32,
    pub uv_max_x: f32,
    pub full_width: f32,
}

#[derive(Component)]
pub struct NameTagHealthbarBackground;

#[derive(Component, Deref, DerefMut)]
pub struct NameTagEntity(pub Entity);
