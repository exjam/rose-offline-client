use bevy::prelude::Resource;

#[derive(Resource)]
pub struct Account {
    pub username: String,
    pub password: String,
}
