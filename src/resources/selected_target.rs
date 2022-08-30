use bevy::prelude::Entity;

#[derive(Default)]
pub struct SelectedTarget {
    pub selected: Option<Entity>,
    pub hover: Option<Entity>,
}
