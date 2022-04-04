use bevy::prelude::Component;

#[derive(Component)]
pub struct DamageDigits {
    pub damage: u32,
    pub model_height: f32,
    // TODO: Critical
    // TODO: Miss
}
