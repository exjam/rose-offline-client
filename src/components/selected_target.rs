use bevy::prelude::{Component, Entity};

#[derive(Component)]
pub struct SelectedTarget {
    pub entity: Entity,
}

impl SelectedTarget {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}
