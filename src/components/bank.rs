use bevy::ecs::prelude::Component;

use rose_data::Item;

#[derive(Component)]
pub struct Bank {
    pub slots: Vec<Option<Item>>,
}
