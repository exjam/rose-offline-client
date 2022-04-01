use bevy::{math::*, prelude::*, render::primitives::Aabb};
use num_derive::FromPrimitive;

#[derive(Copy, Clone, FromPrimitive)]
pub enum ParticleRenderBillboardType {
    /// Does not rotate to face camera
    None = 0,

    /// Rotates only around the y-axis to face camera
    YAxis = 1,

    /// Rotates on all axis to face camera
    Full = 2,
}

#[derive(Component)]
pub struct ParticleRenderData {
    pub positions: Vec<Vec4>,
    pub colors: Vec<Vec4>,
    pub sizes: Vec<Vec2>,
    pub textures: Vec<Vec4>,
    pub blend_op: u8,
    pub src_blend_factor: u8,
    pub dst_blend_factor: u8,
    pub billboard_type: ParticleRenderBillboardType,
}

impl ParticleRenderData {
    pub fn new(
        capacity: usize,
        blend_op: u8,
        src_blend_factor: u8,
        dst_blend_factor: u8,
        billboard_type: ParticleRenderBillboardType,
    ) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            colors: Vec::with_capacity(capacity),
            sizes: Vec::with_capacity(capacity),
            textures: Vec::with_capacity(capacity),
            blend_op,
            src_blend_factor,
            dst_blend_factor,
            billboard_type,
        }
    }

    #[inline(always)]
    pub fn add(&mut self, position: Vec3, rotation: f32, size: Vec2, color: Vec4, texture: Vec4) {
        self.positions.push(Vec4::from((position, rotation)));
        self.colors.push(color);
        self.sizes.push(size);
        self.textures.push(texture);
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
        self.sizes.clear();
        self.textures.clear();
    }

    pub fn compute_aabb(&self) -> Option<Aabb> {
        if self.positions.is_empty() {
            return None;
        }

        let mut min = Vec4::splat(f32::MAX);
        let mut max = Vec4::splat(f32::MIN);
        for (position, size) in self.positions.iter().zip(self.sizes.iter()) {
            let size_vec = Vec4::new(size.x, size.y, size.x, 0.0);
            min = min.min(*position - size_vec);
            max = max.max(*position + size_vec);
        }
        Some(Aabb::from_min_max(min.xyz(), max.xyz()))
    }
}
