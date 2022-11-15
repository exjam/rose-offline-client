use bevy::prelude::Resource;

#[derive(Resource)]
pub struct WorldRates {
    pub craft_rate: i32,
    pub world_price_rate: i32,
    pub item_price_rate: i32,
    pub town_price_rate: i32,
}
