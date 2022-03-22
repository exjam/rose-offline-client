use rose_data::ZoneId;

pub enum GameConnectionEvent {
    JoiningZone(ZoneId),
    JoinedZone(ZoneId),
}
