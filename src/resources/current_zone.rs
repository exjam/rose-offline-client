use rose_data::ZoneId;
use rose_file_readers::HimFile;

pub struct CurrentZone {
    pub id: ZoneId,
    pub grid_per_patch: f32,
    pub grid_size: f32,
    pub heightmaps: Vec<Option<HimFile>>,
}

impl CurrentZone {
    pub fn get_terrain_height(&self, x: f32, y: f32) -> f32 {
        let block_x = x / (16.0 * self.grid_per_patch * self.grid_size);
        let block_y = 65.0 - (y / (16.0 * self.grid_per_patch * self.grid_size));

        if let Some(Some(heightmap)) = self
            .heightmaps
            .get(block_x.max(0.0).min(64.0) as usize + block_y.max(0.0).min(64.0) as usize * 64)
        {
            let tile_x = (heightmap.width - 1) as f32 * block_x.fract();
            let tile_y = (heightmap.height - 1) as f32 * block_y.fract();

            let tile_index_x = tile_x as i32;
            let tile_index_y = tile_y as i32;

            let height_00 = heightmap.get_clamped(tile_index_x, tile_index_y);
            let height_01 = heightmap.get_clamped(tile_index_x, tile_index_y + 1);
            let height_10 = heightmap.get_clamped(tile_index_x + 1, tile_index_y);
            let height_11 = heightmap.get_clamped(tile_index_x + 1, tile_index_y + 1);

            let weight_x = tile_x.fract();
            let weight_y = tile_y.fract();

            let height_y0 = height_00 * (1.0 - weight_x) + height_10 * weight_x;
            let height_y1 = height_01 * (1.0 - weight_x) + height_11 * weight_x;

            height_y0 * (1.0 - weight_y) + height_y1 * weight_y
        } else {
            0.0
        }
    }
}
