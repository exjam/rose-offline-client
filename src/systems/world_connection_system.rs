use bevy::prelude::{Commands, EventWriter, Res, ResMut, State};
use rose_game_common::messages::{client::ClientMessage, server::ServerMessage};
use rose_network_common::ConnectionError;

use crate::{
    events::WorldConnectionEvent,
    resources::{Account, AppState, CharacterList, NetworkThread, WorldConnection},
};

pub fn world_connection_system(
    mut commands: Commands,
    world_connection: Option<Res<WorldConnection>>,
    account: Option<Res<Account>>,
    network_thread: Res<NetworkThread>,
    mut app_state: ResMut<State<AppState>>,
    mut world_connection_events: EventWriter<WorldConnectionEvent>,
) {
    if world_connection.is_none() {
        return;
    }

    let world_connection = world_connection.unwrap();
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
                Ok(server_info) => {
                    commands.insert_resource(network_thread.connect_game(
                        &server_info.ip,
                        server_info.port,
                        server_info.packet_codec_seed,
                        server_info.login_token,
                        account.as_ref().unwrap().password_md5.clone(),
                    ));
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
