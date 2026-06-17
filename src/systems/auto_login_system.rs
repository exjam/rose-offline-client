use bevy::prelude::{Commands, EventWriter, Local, Res, State};

use crate::{
    events::{CharacterSelectEvent, LoginEvent},
    resources::{AppState, AutoLogin, CharacterList, ServerList},
};

#[derive(Default)]
pub enum AutoLoginState {
    #[default]
    Login,
    WaitServerList,
    WaitCharacterList,
    Idle,
}

pub fn auto_login_system(
    mut commands: Commands,
    mut auto_login_state: Local<AutoLoginState>,
    app_state: Res<State<AppState>>,
    character_list: Option<Res<CharacterList>>,
    server_list: Option<Res<ServerList>>,
    auto_login: Option<Res<AutoLogin>>,
    mut login_events: EventWriter<LoginEvent>,
    mut character_select_events: EventWriter<CharacterSelectEvent>,
) {
    if auto_login.is_none() {
        return;
    }

    match *auto_login_state {
        AutoLoginState::Login => {
            if !matches!(app_state.get(), AppState::GameLogin) {
                return;
            }

            if let (Some(username), Some(password)) = (
                auto_login
                    .as_ref()
                    .and_then(|it| it.preset_username.clone()),
                auto_login
                    .as_ref()
                    .and_then(|it| it.preset_password.clone()),
            ) {
                login_events.send(LoginEvent::Login { username, password });
                *auto_login_state = AutoLoginState::WaitServerList;
            }

            if server_list.is_some() {
                // If the user logged in without us, move on to next stage
                *auto_login_state = AutoLoginState::WaitCharacterList;
            }
        }
        AutoLoginState::WaitServerList => {
            if let Some(server_list) = server_list {
                if let (Some(server_id), Some(channel_id)) = (
                    auto_login.as_ref().and_then(|it| it.preset_server_id),
                    auto_login.as_ref().and_then(|it| it.preset_channel_id),
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
            if !matches!(app_state.get(), AppState::GameCharacterSelect) {
                return;
            }

            let preset_character_name = match auto_login
                .as_ref()
                .and_then(|it| it.preset_character_name.as_ref())
            {
                None => {
                    *auto_login_state = AutoLoginState::Idle;
                    return;
                }
                Some(preset_character_name) => preset_character_name,
            };

            let character_list = match character_list.as_ref() {
                None => return,
                Some(character_list) => character_list,
            };

            for (i, character) in character_list.characters.iter().enumerate() {
                if &character.info.name != preset_character_name {
                    continue;
                }

                character_select_events.send(CharacterSelectEvent::SelectCharacter(i));
                character_select_events.send(CharacterSelectEvent::PlaySelected);
                *auto_login_state = AutoLoginState::Idle;
            }
        }
        AutoLoginState::Idle => {
            if !matches!(app_state.get(), AppState::GameCharacterSelect) {
                commands.remove_resource::<AutoLogin>();
                *auto_login_state = AutoLoginState::Login;
            }
        }
    }
}
