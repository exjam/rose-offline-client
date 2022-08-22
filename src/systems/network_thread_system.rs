use bevy::prelude::{Commands, EventReader, Res};

use rose_game_common::{
    data::Password,
    messages::{client::ClientMessage, server::ServerMessage},
};

use crate::{
    events::NetworkEvent,
    protocol::irose,
    resources::{
        GameConnection, LoginConnection, NetworkThread, NetworkThreadMessage, WorldConnection,
    },
};

pub fn network_thread_system(
    mut commands: Commands,
    network_thread: Res<NetworkThread>,
    mut network_events: EventReader<NetworkEvent>,
) {
    for event in network_events.iter() {
        match *event {
            NetworkEvent::ConnectLogin { ref ip, port } => {
                let (server_message_tx, server_message_rx) =
                    crossbeam_channel::unbounded::<ServerMessage>();
                let (client_message_tx, client_message_rx) =
                    tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
                let server_address = format!("{}:{}", ip, port).parse().unwrap();

                network_thread
                    .control_tx
                    .send(NetworkThreadMessage::RunProtocolClient(Box::new(
                        irose::LoginClient::new(
                            server_address,
                            client_message_rx,
                            server_message_tx,
                        ),
                    )))
                    .ok();

                commands
                    .insert_resource(LoginConnection::new(client_message_tx, server_message_rx));
            }
            NetworkEvent::ConnectWorld {
                ref ip,
                port,
                packet_codec_seed,
                login_token,
                ref password,
            } => {
                let (server_message_tx, server_message_rx) =
                    crossbeam_channel::unbounded::<ServerMessage>();
                let (client_message_tx, client_message_rx) =
                    tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
                let server_address = format!("{}:{}", ip, port).parse().unwrap();

                network_thread
                    .control_tx
                    .send(NetworkThreadMessage::RunProtocolClient(Box::new(
                        irose::WorldClient::new(
                            server_address,
                            packet_codec_seed,
                            client_message_rx,
                            server_message_tx,
                        ),
                    )))
                    .ok();

                commands.insert_resource(WorldConnection::new(
                    client_message_tx,
                    server_message_rx,
                    login_token,
                    Password::Plaintext(password.clone()),
                ));
            }
            NetworkEvent::ConnectGame {
                ref ip,
                port,
                packet_codec_seed,
                login_token,
                ref password,
            } => {
                let (server_message_tx, server_message_rx) =
                    crossbeam_channel::unbounded::<ServerMessage>();
                let (client_message_tx, client_message_rx) =
                    tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
                let server_address = format!("{}:{}", ip, port).parse().unwrap();

                network_thread
                    .control_tx
                    .send(NetworkThreadMessage::RunProtocolClient(Box::new(
                        irose::GameClient::new(
                            server_address,
                            packet_codec_seed,
                            client_message_rx,
                            server_message_tx,
                        ),
                    )))
                    .ok();

                commands.insert_resource(GameConnection::new(
                    client_message_tx,
                    server_message_rx,
                    login_token,
                    Password::Plaintext(password.clone()),
                ));
            }
        }
    }
}
