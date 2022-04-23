use std::collections::HashMap;

use rose_game_common::messages::client::ClientMessage;

use crate::scripting::{
    lua4::Lua4Value, quest::quest_check_conditions, LuaUserValueEntity, ScriptFunctionContext,
    ScriptFunctionResources,
};

pub struct LuaQuestFunctions {
    pub closures: HashMap<
        String,
        fn(&ScriptFunctionResources, &mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>,
    >,
}

impl Default for LuaQuestFunctions {
    fn default() -> Self {
        let mut closures: HashMap<
            String,
            fn(
                &ScriptFunctionResources,
                &mut ScriptFunctionContext,
                Vec<Lua4Value>,
            ) -> Vec<Lua4Value>,
        > = HashMap::new();

        closures.insert("QF_checkQuestCondition".into(), QF_checkQuestCondition);
        closures.insert("QF_doQuestTrigger".into(), QF_doQuestTrigger);
        closures.insert("QF_findQuest".into(), QF_findQuest);
        closures.insert("QF_getEventOwner".into(), QF_getEventOwner);
        closures.insert("QF_getEpisodeVAR".into(), QF_getEpisodeVAR);
        closures.insert("QF_getJobVAR".into(), QF_getJobVAR);
        closures.insert("QF_getPlanetVAR".into(), QF_getPlanetVAR);
        closures.insert("QF_getQuestCount".into(), QF_getQuestCount);
        closures.insert("QF_getQuestID".into(), QF_getQuestID);
        closures.insert("QF_getQuestSwitch".into(), QF_getQuestSwitch);
        closures.insert("QF_getQuestVar".into(), QF_getQuestVar);
        closures.insert("QF_getUserSwitch".into(), QF_getUserSwitch);
        closures.insert("QF_getNpcQuestZeroVal".into(), QF_getNpcQuestZeroVal);

        /*
        QF_appendQuest
        QF_beginCon
        QF_CameraworkingNpc
        QF_CameraworkingPoint
        QF_CameraworkingSelf
        QF_ChangetalkImage
        QF_ChangetalkName
        QF_closeCon
        QF_deleteQuest
        QF_EffectCallNpc
        QF_EffectCallSelf
        QF_getQuestItemQuantity
        QF_getSkillLevel
        QF_getUnionVAR
        QF_givePoint
        QF_gotoCon
        QF_MotionCallNpc
        QF_MotionCallSelf
        QF_NpcHide
        QF_NpcTalkinterfaceHide
        QF_NpcTalkinterfaceView
        QF_NpcView
        */

        Self { closures }
    }
}

#[allow(non_snake_case)]
fn QF_checkQuestCondition(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(quest_trigger_name) = parameters[0].to_string() {
        log::trace!(target: "lua", "QF_checkQuestCondition({})", &quest_trigger_name);

        if let Ok(true) =
            quest_check_conditions(resources, context, quest_trigger_name.as_str().into())
        {
            return vec![1.into()];
        }
    }

    vec![0.into()]
}

#[allow(non_snake_case)]
fn QF_doQuestTrigger(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = if let Ok(quest_trigger_name) = parameters[0].to_string() {
        if let Ok(true) =
            quest_check_conditions(resources, context, quest_trigger_name.as_str().into())
        {
            if let Some(game_connection) = resources.game_connection.as_ref() {
                game_connection
                    .client_message_tx
                    .send(ClientMessage::QuestTrigger(
                        quest_trigger_name.as_str().into(),
                    ))
                    .ok();
            }

            1
        } else {
            0
        }
    } else {
        0
    };

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_findQuest(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let quest_id = parameters.get(0)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;
        quest_state
            .find_active_quest_index(quest_id)
            .map(|x| x as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getEventOwner(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(lua_value_entity) = parameters[0].to_user_type::<LuaUserValueEntity>() {
        if let Ok(client_entity) = context.query_client_entity.get(lua_value_entity.entity) {
            return vec![client_entity.id.0.into()];
        }
    }

    vec![0.into()]
}

#[allow(non_snake_case)]
fn QF_getEpisodeVAR(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let var_id = parameters.get(0)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;
        Some(*quest_state.episode_variables.get(var_id)? as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getJobVAR(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let var_id = parameters.get(0)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;
        Some(*quest_state.job_variables.get(var_id)? as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getPlanetVAR(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let var_id = parameters.get(0)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;
        Some(*quest_state.planet_variables.get(var_id)? as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getQuestCount(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    _parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let quest_state = context.query_quest.get_single().ok()?;
        Some(
            quest_state
                .active_quests
                .iter()
                .filter(|x| x.is_some())
                .count() as i32,
        )
    }()
    .unwrap_or(0);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getQuestID(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let quest_index = parameters.get(0)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;
        let quest = quest_state.get_quest(quest_index)?;
        Some(quest.quest_id as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getQuestSwitch(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let quest_index = parameters.get(0)?.to_usize().ok()?;
        let quest_switch_id = parameters.get(1)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;

        let quest = quest_state.get_quest(quest_index)?;
        Some(*quest.switches.get(quest_switch_id)? as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getQuestVar(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let quest_index = parameters.get(0)?.to_usize().ok()?;
        let quest_var_id = parameters.get(1)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;

        let quest = quest_state.get_quest(quest_index)?;
        Some(*quest.variables.get(quest_var_id)? as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getUserSwitch(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let switch_id = parameters.get(0)?.to_usize().ok()?;
        let quest_state = context.query_quest.get_single().ok()?;
        Some(*quest_state.quest_switches.get(switch_id)? as i32)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

#[allow(non_snake_case)]
fn QF_getNpcQuestZeroVal(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let npc_id = parameters.get(0)?.to_usize().ok()?;

        for npc in context.query_npc.iter() {
            if npc.id.get() as usize == npc_id {
                return Some(npc.quest_index as i32);
            }
        }

        None
    }()
    .unwrap_or(0);

    vec![result.into()]
}
