use bevy::{prelude::Component, reflect::Reflect};

use rose_data::WarpGateId;

#[derive(Component, Reflect)]
pub struct WarpObject {
    pub warp_id: WarpGateId,
    pub last_collision: f64,
}

impl WarpObject {
    pub fn new(warp_id: WarpGateId) -> Self {
        Self {
            warp_id,
            last_collision: 0.0,
        }
    }
}
