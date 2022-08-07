use bevy::prelude::{Commands, EventWriter, Res, ResMut, State};
use rose_game_common::messages::{
    client::ClientMessage,
    server::{JoinServerResponse, ServerMessage},
};
use rose_network_common::ConnectionError;

use crate::{
    events::{NetworkEvent, WorldConnectionEvent},
    resources::{Account, AppState, CharacterList, WorldConnection},
};

pub fn world_connection_system(
    mut commands: Commands,
    world_connection: Option<Res<WorldConnection>>,
    account: Option<Res<Account>>,
    mut app_state: ResMut<State<AppState>>,
    mut network_events: EventWriter<NetworkEvent>,
    mut world_connection_events: EventWriter<WorldConnectionEvent>,
) {
    let world_connection = if let Some(world_connection) = world_connection {
        world_connection
    } else {
        return;
    };

    let account = if let Some(account) = account {
        account
    } else {
        return;
    };

    let result: Result<(), anyhow::Error> = loop {
        match world_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionResponse(response)) => match response {
                Ok(_) => {
                    world_connection
                        .client_message_tx
                        .send(ClientMessage::GetCharacterList)
                        .ok();
                }
                Err(_) => {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            },
            Ok(ServerMessage::CharacterList(characters)) => {
                if !matches!(app_state.current(), AppState::GameCharacterSelect) {
                    app_state.set(AppState::GameCharacterSelect).ok();
                }

                commands.insert_resource(CharacterList { characters });
            }
            Ok(ServerMessage::SelectCharacter(response)) => match response {
                Ok(JoinServerResponse {
                    login_token,
                    packet_codec_seed,
                    ip,
                    port,
                }) => {
                    network_events.send(NetworkEvent::ConnectGame {
                        ip,
                        port,
                        packet_codec_seed,
                        login_token,
                        password: account.password.clone(),
                    });
                }
                Err(_) => {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            },
            Ok(ServerMessage::CreateCharacter(response)) => {
                world_connection_events
                    .send(WorldConnectionEvent::CreateCharacterResponse(response));
            }
            Ok(ServerMessage::DeleteCharacter(response)) => {
                world_connection_events
                    .send(WorldConnectionEvent::DeleteCharacterResponse(response));
            }
            // ServerMessage::ReturnToCharacterSelect
            Ok(message) => {
                log::warn!("Received unexpected world server message: {:#?}", message);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => break Ok(()),
        }
    };

    if let Err(error) = result {
        // TODO: Store error somewhere to display to user
        log::warn!("World server connection error: {}", error);
        commands.remove_resource::<WorldConnection>();
    }
}
