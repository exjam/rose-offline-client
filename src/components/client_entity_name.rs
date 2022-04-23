use bevy::prelude::Component;

#[derive(Component)]
pub struct ClientEntityName {
    pub name: String,
}

impl ClientEntityName {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
