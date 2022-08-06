#[allow(clippy::enum_variant_names)]
pub enum NetworkEvent {
    ConnectLogin {
        ip: String,
        port: u16,
    },
    ConnectWorld {
        ip: String,
        port: u16,
        packet_codec_seed: u32,
        login_token: u32,
        password: String,
    },
    ConnectGame {
        ip: String,
        port: u16,
        packet_codec_seed: u32,
        login_token: u32,
        password: String,
    },
}
