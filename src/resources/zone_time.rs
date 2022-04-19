#[derive(Copy, Clone, Debug)]
pub enum ZoneTimeState {
    Morning,
    Day,
    Evening,
    Night,
}

pub struct ZoneTime {
    pub state: ZoneTimeState,
    pub state_percent_complete: f32,
    pub time: u32,
    pub day_cycle: u32,
    pub morning_time: u32,
    pub day_time: u32,
    pub evening_time: u32,
    pub night_time: u32,
    pub debug_overwrite_time: Option<u32>,
}

impl Default for ZoneTime {
    fn default() -> Self {
        Self {
            state: ZoneTimeState::Morning,
            state_percent_complete: 0.0,
            time: 0,
            day_cycle: 160,
            morning_time: 0,
            day_time: 10,
            evening_time: 111,
            night_time: 124,
            debug_overwrite_time: None,
        }
    }
}
