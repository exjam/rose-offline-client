use bevy::{
    prelude::{Component, Vec3},
    reflect::Reflect,
};

#[derive(Component, Default, Reflect)]
pub struct FacingDirection {
    pub desired: f32,
    pub actual: f32,
}

impl FacingDirection {
    pub fn set_desired_vector(&mut self, direction: Vec3) {
        self.desired = direction.y.atan2(direction.x) + std::f32::consts::PI;
    }
}
