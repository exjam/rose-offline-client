use bevy::prelude::{Handle, Image, Resource};

#[derive(Resource)]
pub struct SpecularTexture {
    pub image: Handle<Image>,
}
