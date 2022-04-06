use bevy::prelude::{App, Entity, Plugin};

pub mod lua4;

mod lua_game_constants;
mod lua_game_functions;
mod lua_quest_functions;
mod quest;
mod quest_condition_functions;
mod quest_function_context;
mod quest_reward_functions;
mod script_function_context;
mod script_function_resources;

pub struct LuaUserValueEntity {
    pub entity: Entity,
}

pub use lua_game_constants::LuaGameConstants;
pub use lua_game_functions::LuaGameFunctions;
pub use lua_quest_functions::LuaQuestFunctions;
pub use quest::{quest_apply_rewards, quest_check_conditions};
pub use quest_condition_functions::quest_trigger_check_conditions;
pub use quest_function_context::QuestFunctionContext;
pub use quest_reward_functions::{quest_triggers_apply_rewards, quest_triggers_skip_rewards};
pub use script_function_context::ScriptFunctionContext;
pub use script_function_resources::ScriptFunctionResources;

#[derive(Default)]
pub struct RoseScriptingPlugin;

impl Plugin for RoseScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LuaGameConstants>();
        app.init_resource::<LuaGameFunctions>();
        app.init_resource::<LuaQuestFunctions>();
    }
}
