use crate::{components::NameTagType, render::WorldUiRect, systems::MAX_NAME_ROWS};
use arrayvec::ArrayVec;
use bevy::{
    prelude::{Color, Entity, Handle, Image, Resource, Vec2},
    utils::HashMap,
};
use std::sync::Arc;

pub struct NameTagData {
    pub image: Handle<Image>,
    pub size: Vec2,
    pub rects: ArrayVec<WorldUiRect, MAX_NAME_ROWS>,
}

pub struct NameTagPendingData {
    pub galley: Arc<egui::Galley>,
    pub colors: ArrayVec<Color, MAX_NAME_ROWS>,
    pub name_tag_type: NameTagType,
}

#[derive(Default, Resource)]
pub struct NameTagCache {
    pub cache: HashMap<String, NameTagData>,
    pub pending: HashMap<Entity, NameTagPendingData>,
    pub pixels_per_point: f32,
    pub dispose: bool,
}
