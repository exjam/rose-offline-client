use bevy::prelude::{App, Entity, Plugin};

pub mod lua4;

mod lua_game_constants;
mod lua_game_functions;
mod lua_quest_functions;
mod script_function_context;

pub struct LuaUserValueEntity {
    pub entity: Entity,
}

pub use lua_game_constants::LuaGameConstants;
pub use lua_game_functions::LuaGameFunctions;
pub use lua_quest_functions::LuaQuestFunctions;
pub use script_function_context::ScriptFunctionContext;

#[derive(Default)]
pub struct RoseScriptingPlugin;

impl Plugin for RoseScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LuaGameConstants>();
        app.init_resource::<LuaGameFunctions>();
        app.init_resource::<LuaQuestFunctions>();
    }
}
