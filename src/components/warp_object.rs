use bevy::prelude::Component;
use rose_data::WarpGateId;

#[derive(Component)]
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
