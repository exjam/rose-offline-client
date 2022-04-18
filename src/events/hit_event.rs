use bevy::prelude::Entity;

pub struct HitEvent {
    pub attacker: Entity,
    pub defender: Entity,
}

impl HitEvent {
    pub fn new(attacker: Entity, defender: Entity) -> Self {
        Self { attacker, defender }
    }
}
