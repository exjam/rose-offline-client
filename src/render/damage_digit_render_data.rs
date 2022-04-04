use bevy::{math::*, prelude::*};

#[derive(Component)]
pub struct DamageDigitRenderData {
    pub positions: Vec<Vec4>,
    pub sizes: Vec<Vec2>,
    pub uvs: Vec<Vec4>,
}

impl DamageDigitRenderData {
    pub fn new(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            sizes: Vec::with_capacity(capacity),
            uvs: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn add(&mut self, position: Vec3, digit_x_offset: f32, size: Vec2, uv: Vec4) {
        self.positions.push(Vec4::from((position, digit_x_offset)));
        self.sizes.push(size);
        self.uvs.push(uv);
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.sizes.clear();
        self.uvs.clear();
    }
}
