use bevy::prelude::{Commands, Res, ResMut};
use rose_game_common::messages::{
    client::{ClientMessage, GetChannelList, LoginRequest},
    server::{ChannelList, JoinServerResponse, LoginResponse, ServerMessage},
};
use rose_network_common::ConnectionError;

use crate::resources::{
    Account, LoginConnection, NetworkThread, ServerList, ServerListGameServer,
    ServerListWorldServer,
};

pub fn login_connection_system(
    mut commands: Commands,
    account: Option<Res<Account>>,
    login_connection: Option<Res<LoginConnection>>,
    network_thread: Res<NetworkThread>,
    mut server_list: Option<ResMut<ServerList>>,
) {
    if login_connection.is_none() {
        return;
    }

    let login_connection = login_connection.unwrap();
    let result: Result<(), anyhow::Error> = loop {
        match login_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionResponse(response)) => match response {
                Ok(_) => {
                    if let Some(account) = account.as_ref() {
                        login_connection
                            .client_message_tx
                            .send(ClientMessage::LoginRequest(LoginRequest {
                                username: account.username.clone(),
                                password_md5: account.password_md5.clone(),
                            }))
                            .ok();
                    } else {
                        break Err(ConnectionError::ConnectionLost.into());
                    }
                }
                Err(_) => {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            },
            Ok(ServerMessage::LoginResponse(response)) => match response {
                Ok(LoginResponse { server_list }) => {
                    let mut world_servers = Vec::new();
                    for (id, name) in server_list {
                        login_connection
                            .client_message_tx
                            .send(ClientMessage::GetChannelList(GetChannelList {
                                server_id: id as usize,
                            }))
                            .ok();
                        world_servers.push(ServerListWorldServer {
                            id: id as usize,
                            name,
                            game_servers: Vec::new(),
                        });
                    }
                    commands.insert_resource(ServerList { world_servers });
                }
                Err(error) => {
                    break Err(error.into());
                }
            },
            Ok(ServerMessage::ChannelList(response)) => {
                if let Ok(ChannelList {
                    server_id,
                    channels,
                }) = response
                {
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
            }
            Ok(ServerMessage::JoinServer(response)) => match response {
                Ok(JoinServerResponse {
                    login_token,
                    packet_codec_seed,
                    ip,
                    port,
                }) => {
                    if let Some(account) = account.as_ref() {
                        commands.insert_resource(network_thread.connect_world(
                            &ip,
                            port,
                            packet_codec_seed,
                            login_token,
                            account.password_md5.clone(),
                        ));
                    } else {
                        break Err(ConnectionError::ConnectionLost.into());
                    }
                }
                Err(error) => break Err(error.into()),
            },
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
