use arrayvec::ArrayVec;
use bevy::{
    prelude::{Handle, Image, Resource, Vec2},
    utils::HashMap,
};

use crate::render::WorldUiRect;

pub struct NameTagData {
    pub image: Handle<Image>,
    pub size: Vec2,
    pub rects: ArrayVec<WorldUiRect, 2>, // NPC names are 2 rows
}

#[derive(Default, Resource)]
pub struct NameTagCache {
    pub cache: HashMap<String, NameTagData>,
}
