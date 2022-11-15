use bevy::prelude::Resource;

#[derive(Resource)]
pub enum LoginState {
    Input,
    WaitServerList,
    ServerSelect,
    JoiningServer,
}
