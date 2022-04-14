use bevy::prelude::{Component, Entity};

#[derive(Component)]
pub struct Projectile {
    pub source: Entity,
}
