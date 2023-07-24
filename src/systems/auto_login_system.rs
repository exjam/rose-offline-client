use bevy::prelude::{EventWriter, Local, Res, State};

use crate::{
    events::{CharacterSelectEvent, LoginEvent},
    resources::{AppState, CharacterList, ServerConfiguration, ServerList},
};

#[derive(Default)]
pub enum AutoLoginState {
    #[default]
    Login,
    WaitServerList,
    WaitCharacterList,
    SelectedCharacter,
}

pub fn auto_login_system(
    mut auto_login_state: Local<AutoLoginState>,
    app_state: Res<State<AppState>>,
    character_list: Option<Res<CharacterList>>,
    server_list: Option<Res<ServerList>>,
    server_configuration: Res<ServerConfiguration>,
    mut login_events: EventWriter<LoginEvent>,
    mut character_select_events: EventWriter<CharacterSelectEvent>,
) {
    if !server_configuration.auto_login {
        return;
    }

    match *auto_login_state {
        AutoLoginState::Login => {
            if matches!(app_state.get(), AppState::GameLogin) {
                if let (Some(username), Some(password)) = (
                    &server_configuration.preset_username,
                    &server_configuration.preset_password,
                ) {
                    login_events.send(LoginEvent::Login {
                        username: username.clone(),
                        password: password.clone(),
                    });
                    *auto_login_state = AutoLoginState::WaitServerList;
                }

                if server_list.is_some() {
                    // If the user logged in without us, move on to next stage
                    *auto_login_state = AutoLoginState::WaitCharacterList;
                }
            }
        }
        AutoLoginState::WaitServerList => {
            if let Some(server_list) = server_list {
                if let (&Some(server_id), &Some(channel_id)) = (
                    &server_configuration.preset_server_id,
                    &server_configuration.preset_channel_id,
                ) {
                    for world_server in server_list.world_servers.iter() {
                        if world_server.id == server_id {
                            for game_server in world_server.game_servers.iter() {
                                if game_server.id == channel_id {
                                    login_events.send(LoginEvent::SelectServer {
                                        server_id,
                                        channel_id,
                                    });
                                    *auto_login_state = AutoLoginState::WaitCharacterList;
                                }
                            }
                        }
                    }
                } else if server_list.world_servers.len() == 1
                    && server_list.world_servers[0].game_servers.len() == 1
                {
                    login_events.send(LoginEvent::SelectServer {
                        server_id: server_list.world_servers[0].id,
                        channel_id: server_list.world_servers[0].game_servers[0].id,
                    });
                    *auto_login_state = AutoLoginState::WaitCharacterList;
                }
            }

            if matches!(app_state.get(), AppState::GameCharacterSelect) {
                *auto_login_state = AutoLoginState::WaitCharacterList;
            }
        }
        AutoLoginState::WaitCharacterList => {
            if matches!(app_state.get(), AppState::GameCharacterSelect) {
                if let Some(preset_character_name) =
                    server_configuration.preset_character_name.as_ref()
                {
                    if let Some(character_list) = character_list.as_ref() {
                        for (i, character) in character_list.characters.iter().enumerate() {
                            if &character.info.name == preset_character_name {
                                character_select_events
                                    .send(CharacterSelectEvent::SelectCharacter(i));
                                character_select_events.send(CharacterSelectEvent::PlaySelected);
                                *auto_login_state = AutoLoginState::SelectedCharacter;
                            }
                        }
                    }
                }
            }
        }
        AutoLoginState::SelectedCharacter => {}
    }
}
