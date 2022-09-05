use std::collections::HashMap;

use rose_game_common::{components::CharacterGender, messages::ClientEntityId};

use crate::{
    events::{BankEvent, NpcStoreEvent},
    scripting::{
        lua4::Lua4Value,
        lua_game_constants::{
            SV_BIRTH, SV_CHA, SV_CLASS, SV_CON, SV_DEX, SV_EXP, SV_FAME, SV_INT, SV_LEVEL, SV_RANK,
            SV_SEN, SV_SEX, SV_STR, SV_UNION,
        },
        ScriptFunctionContext, ScriptFunctionResources,
    },
};

pub struct LuaGameFunctions {
    pub closures: HashMap<
        String,
        fn(&ScriptFunctionResources, &mut ScriptFunctionContext, Vec<Lua4Value>) -> Vec<Lua4Value>,
    >,
}

impl Default for LuaGameFunctions {
    fn default() -> Self {
        let mut closures: HashMap<
            String,
            fn(
                &ScriptFunctionResources,
                &mut ScriptFunctionContext,
                Vec<Lua4Value>,
            ) -> Vec<Lua4Value>,
        > = HashMap::new();

        closures.insert("GF_getVariable".into(), GF_getVariable);
        closures.insert("GF_openBank".into(), GF_openBank);
        closures.insert("GF_openStore".into(), GF_openStore);

        /*
        GF_addUserMoney
        GF_appraisal
        GF_ChangeState
        GF_checkNumOfInvItem
        GF_checkTownItem
        GF_checkUserMoney
        GF_DeleteEffectFromObject
        GF_disorganizeClan
        GF_EffectOnObject
        GF_error
        GF_getDate
        GF_GetEffectUseFile
        GF_GetEffectUseIndex
        GF_getGameVersion
        GF_getIDXOfInvItem
        GF_getItemRate
        GF_GetMotionUseFile
        GF_getName
        GF_getReviveZoneName
        GF_GetTarget
        GF_getTownRate
        GF_getTownVar
        GF_getWorldRate
        GF_getZone
        GF_giveEquipItemIntoInv
        GF_giveUsableItemIntoInv
        GF_log
        GF_LogString
        GF_movableXY
        GF_moveEvent
        GF_moveXY
        GF_openDeliveryStore
        GF_openSeparate
        GF_openUpgrade
        GF_organizeClan
        GF_playEffect
        GF_playSound
        GF_putoffItem
        GF_putonItem
        GF_Random
        GF_repair
        GF_rotateCamera
        GF_setEquipedItem
        GF_SetMotion
        GF_setRevivePosition
        GF_setTownRate
        GF_setVariable
        GF_setWorldRate
        GF_spawnMonAtEvent
        GF_spawnMonXY
        GF_takeItemFromInv
        GF_takeUserMoney
        GF_warp
        GF_WeatherEffectOnObject
        GF_zoomCamera
        */

        Self { closures }
    }
}

#[allow(non_snake_case)]
fn GF_getVariable(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let variable_id = parameters[0].to_i32().unwrap();
    let character = context.query_player.single();

    let value = match variable_id {
        SV_SEX => match character.character_info.gender {
            CharacterGender::Male => 0,
            CharacterGender::Female => 1,
        },
        SV_BIRTH => character.character_info.birth_stone as i32,
        SV_CLASS => character.character_info.job as i32,
        SV_UNION => character
            .union_membership
            .current_union
            .map(|x| x.get() as i32)
            .unwrap_or(0),
        SV_RANK => character.character_info.rank as i32,
        SV_FAME => character.character_info.fame as i32,
        SV_STR => character.basic_stats.strength,
        SV_DEX => character.basic_stats.dexterity,
        SV_INT => character.basic_stats.intelligence,
        SV_CON => character.basic_stats.concentration,
        SV_CHA => character.basic_stats.charm,
        SV_SEN => character.basic_stats.sense,
        SV_EXP => character.experience_points.xp as i32,
        SV_LEVEL => character.level.level as i32,
        _ => 0,
    };

    vec![value.into()]
}

#[allow(non_snake_case)]
fn GF_openBank(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    (|| -> Option<()> {
        let client_entity_id = ClientEntityId(parameters.get(0)?.to_usize().ok()?);

        context
            .bank_events
            .send(BankEvent::OpenBankFromClientEntity { client_entity_id });

        Some(())
    })();

    vec![]
}

#[allow(non_snake_case)]
fn GF_openStore(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    (|| -> Option<()> {
        let npc_client_entity_id = ClientEntityId(parameters.get(0)?.to_usize().ok()?);
        context
            .npc_store_events
            .send(NpcStoreEvent::OpenClientEntityStore(npc_client_entity_id));
        Some(())
    })();
    vec![]
}
