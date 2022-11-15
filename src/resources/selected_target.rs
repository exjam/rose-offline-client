use bevy::prelude::{Entity, Resource};

#[derive(Default, Resource)]
pub struct SelectedTarget {
    pub selected: Option<Entity>,
    pub hover: Option<Entity>,
}
