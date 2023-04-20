use bevy::prelude::{Commands, EventWriter, Res, ResMut};

use rose_game_common::{
    data::Password,
    messages::{client::ClientMessage, server::ServerMessage},
};
use rose_network_common::ConnectionError;

use crate::{
    events::NetworkEvent,
    resources::{
        Account, LoginConnection, ServerList, ServerListGameServer, ServerListWorldServer,
    },
};

pub fn login_connection_system(
    mut commands: Commands,
    account: Option<Res<Account>>,
    login_connection: Option<Res<LoginConnection>>,
    mut server_list: Option<ResMut<ServerList>>,
    mut network_events: EventWriter<NetworkEvent>,
) {
    if login_connection.is_none() {
        return;
    }

    let login_connection = login_connection.unwrap();
    let result: Result<(), anyhow::Error> = loop {
        match login_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionRequestSuccess {
                packet_sequence_id: _,
            }) => {
                if let Some(account) = account.as_ref() {
                    login_connection
                        .client_message_tx
                        .send(ClientMessage::LoginRequest {
                            username: account.username.clone(),
                            password: Password::Plaintext(account.password.clone()),
                        })
                        .ok();
                } else {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            }
            Ok(ServerMessage::ConnectionRequestError { error: _ }) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Ok(ServerMessage::LoginSuccess { server_list }) => {
                let mut world_servers = Vec::new();
                for (id, name) in server_list {
                    login_connection
                        .client_message_tx
                        .send(ClientMessage::GetChannelList {
                            server_id: id as usize,
                        })
                        .ok();
                    world_servers.push(ServerListWorldServer {
                        id: id as usize,
                        name,
                        game_servers: Vec::new(),
                    });
                }
                commands.insert_resource(ServerList { world_servers });
            }
            Ok(ServerMessage::LoginError { error }) => {
                break Err(error.into());
            }
            Ok(ServerMessage::ChannelList {
                server_id,
                channels,
            }) => {
                let mut game_servers = Vec::new();
                for (id, name) in channels {
                    game_servers.push(ServerListGameServer {
                        id: id as usize,
                        name,
                    });
                }

                if let Some(server_list) = server_list.as_mut() {
                    for world_server in server_list.world_servers.iter_mut() {
                        if world_server.id == server_id {
                            world_server.game_servers = game_servers;
                            break;
                        }
                    }
                }
            }
            Ok(ServerMessage::JoinServerSuccess {
                login_token,
                packet_codec_seed,
                ip,
                port,
            }) => {
                if let Some(account) = account.as_ref() {
                    network_events.send(NetworkEvent::ConnectWorld {
                        ip,
                        port,
                        packet_codec_seed,
                        login_token,
                        password: account.password.clone(),
                    });
                } else {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            }
            Ok(ServerMessage::JoinServerError { error }) => {
                break Err(error.into());
            }
            Ok(message) => {
                log::warn!("Received unexpected login server message: {:#?}", message);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => break Ok(()),
        }
    };

    if let Err(error) = result {
        // TODO: Store error somewhere to display to user
        log::warn!("Login server connection error: {}", error);
        commands.remove_resource::<LoginConnection>();
    }
}
