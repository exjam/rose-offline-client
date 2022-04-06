use std::sync::Arc;

use bevy::{
    math::Vec3Swizzles,
    prelude::{Entity, EventReader, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};
use rose_file_readers::{ConFile, ConMessageType};

use crate::{
    components::{PlayerCharacter, Position},
    events::ConversationDialogEvent,
    resources::GameData,
    scripting::{
        lua4::{Lua4Function, Lua4VM, Lua4VMError, Lua4VMRustClosures, Lua4Value},
        LuaGameConstants, LuaGameFunctions, LuaQuestFunctions, LuaUserValueEntity,
        ScriptFunctionContext, ScriptFunctionResources,
    },
    VfsResource,
};

pub struct GeneratedDialogResponse {
    pub text: String,
    pub action_function: String,
    pub menu_index: i32,
}

#[derive(Default)]
pub struct GeneratedDialog {
    pub message: String,
    pub responses: Vec<GeneratedDialogResponse>,
}

pub struct ConversationDialogState {
    pub npc_entity: Entity,
    pub con_file: ConFile,
    pub generated_dialog: GeneratedDialog,
    pub lua_vm: Lua4VM,
    pub event_object_handle: Arc<dyn std::any::Any + Send + Sync>,
}

pub struct LuaVMContext<'a, 'w1, 's1, 'w2, 's2> {
    pub function_context: &'a mut ScriptFunctionContext<'w1, 's1>,
    pub function_resources: &'a ScriptFunctionResources<'w2, 's2>,
    pub game_constants: &'a LuaGameConstants,
    pub game_functions: &'a LuaGameFunctions,
    pub quest_functions: &'a LuaQuestFunctions,
}

impl<'a, 'w1, 's1, 'w2, 's2> Lua4VMRustClosures for LuaVMContext<'a, 'w1, 's1, 'w2, 's2> {
    fn call_rust_closure(
        &mut self,
        name: &str,
        parameters: Vec<Lua4Value>,
    ) -> Result<Vec<Lua4Value>, Lua4VMError> {
        if let Some(closure) = self.quest_functions.closures.get(name) {
            Ok(closure(
                self.function_resources,
                self.function_context,
                parameters,
            ))
        } else if let Some(closure) = self.game_functions.closures.get(name) {
            Ok(closure(
                self.function_resources,
                self.function_context,
                parameters,
            ))
        } else {
            Err(Lua4VMError::GlobalNotFound(name.to_string()))
        }
    }
}

fn create_conversation_dialog(
    con_file: ConFile,
    user_context: &mut LuaVMContext,
    npc_entity: Entity,
) -> Option<ConversationDialogState> {
    let mut lua_vm = Lua4VM::new();

    for (name, value) in user_context.game_constants.constants.iter() {
        lua_vm.set_global(name.clone(), value.clone());
    }

    for (name, _) in user_context.game_functions.closures.iter() {
        lua_vm.set_global(name.clone(), Lua4Value::RustClosure(name.clone()));
    }

    for (name, _) in user_context.quest_functions.closures.iter() {
        lua_vm.set_global(name.clone(), Lua4Value::RustClosure(name.clone()));
    }

    let lua_function = Lua4Function::from_bytes(&con_file.script_binary).ok()?;
    lua_vm
        .call_lua_function(user_context, &lua_function, &[])
        .ok()?;

    Some(ConversationDialogState {
        npc_entity,
        con_file,
        event_object_handle: Arc::new(LuaUserValueEntity { entity: npc_entity }),
        generated_dialog: Default::default(),
        lua_vm,
    })
}

impl GeneratedDialog {
    fn run_menu(
        &mut self,
        lua_vm: &mut Lua4VM,
        user_context: &mut LuaVMContext,
        con_file: &ConFile,
        event_object_handle: &Arc<dyn std::any::Any + Send + Sync>,
        game_data: &GameData,
        menu_idx: i32,
    ) -> bool {
        if menu_idx < 0 {
            return false;
        }

        let menu = &con_file.menus[menu_idx as usize];
        let mut any_added = false;
        for message in menu.messages.iter() {
            if !message.condition_function.is_empty() {
                match lua_vm.call_global_closure(
                    user_context,
                    &message.condition_function,
                    &[Lua4Value::UserData(event_object_handle.clone())],
                ) {
                    Ok(result) => {
                        let result = result
                            .get(0)
                            .and_then(|value| value.to_i32().ok())
                            .unwrap_or(0);

                        if result == 0 {
                            log::debug!(
                                "Menu check function {} failed with result: {}",
                                &message.condition_function,
                                result
                            );
                            continue;
                        }
                    }
                    Err(error) => {
                        log::error!(
                            "Error running conversation script function {}: {}",
                            &message.condition_function,
                            error
                        );
                        continue;
                    }
                }
            }

            match message.message_type {
                ConMessageType::Close
                | ConMessageType::PlayerSelect
                | ConMessageType::JumpSelect => {
                    self.responses.push(GeneratedDialogResponse {
                        text: game_data
                            .ltb_event
                            .get_string(message.string_id as usize, 2)
                            .unwrap_or_else(|| "???".into()),
                        action_function: message.action_function.clone(),
                        menu_index: message.message_value,
                    });
                }
                ConMessageType::NextMessage | ConMessageType::ShowMessage => {
                    self.message = game_data
                        .ltb_event
                        .get_string(message.string_id as usize, 2)
                        .unwrap_or_else(|| "???".into());
                    self.responses.clear();

                    self.run_menu(
                        lua_vm,
                        user_context,
                        con_file,
                        event_object_handle,
                        game_data,
                        message.message_value,
                    );
                }
            }

            any_added = true;
        }

        any_added
    }
}

pub fn conversation_dialog_system(
    mut current_dialog_state: Local<Option<ConversationDialogState>>,
    mut egui_context: ResMut<EguiContext>,
    mut conversation_dialog_events: EventReader<ConversationDialogEvent>,
    mut lua_function_context: ScriptFunctionContext,
    script_function_resources: ScriptFunctionResources,
    query_player_position: Query<&Position, With<PlayerCharacter>>,
    query_position: Query<&Position>,
    lua_game_constants: Res<LuaGameConstants>,
    lua_game_functions: Res<LuaGameFunctions>,
    lua_quest_functions: Res<LuaQuestFunctions>,
    game_data: Res<GameData>,
    vfs_resource: Res<VfsResource>,
) {
    let mut user_context = LuaVMContext {
        function_context: &mut lua_function_context,
        function_resources: &script_function_resources,
        game_constants: &lua_game_constants,
        game_functions: &lua_game_functions,
        quest_functions: &lua_quest_functions,
    };

    for event in conversation_dialog_events.iter() {
        let ConversationDialogEvent::OpenNpcDialog(npc_entity, con_file_path) = event;
        *current_dialog_state = None;

        if let Some(mut next_dialog_state) = vfs_resource
            .vfs
            .read_file::<ConFile, _>(con_file_path)
            .ok()
            .and_then(|con_file| {
                create_conversation_dialog(con_file, &mut user_context, *npc_entity)
            })
        {
            let check_open_function =
                &next_dialog_state.con_file.initial_messages[0].condition_function;

            if !check_open_function.is_empty() {
                match next_dialog_state.lua_vm.call_global_closure(
                    &mut user_context,
                    check_open_function,
                    &[Lua4Value::UserData(
                        next_dialog_state.event_object_handle.clone(),
                    )],
                ) {
                    Ok(result) => {
                        let result = result
                            .get(0)
                            .and_then(|value| value.to_i32().ok())
                            .unwrap_or(0);
                        if result < 1 {
                            log::debug!(
                                "Conversation check open function {} failed with value {}",
                                check_open_function,
                                result
                            );
                        } else {
                            // Success, open dialog
                            if next_dialog_state.generated_dialog.run_menu(
                                &mut next_dialog_state.lua_vm,
                                &mut user_context,
                                &next_dialog_state.con_file,
                                &next_dialog_state.event_object_handle,
                                &game_data,
                                0,
                            ) {
                                *current_dialog_state = Some(next_dialog_state);
                            }
                        }
                    }
                    Err(error) => {
                        log::error!(
                            "Error running conversation open script function {}: {}",
                            check_open_function,
                            error
                        );
                    }
                }
            }
        }
    }

    if let Some(dialog_state) = current_dialog_state.as_mut() {
        let mut selected_response = None;
        let mut open = true;

        // If player has moved away from NPC, close the dialog
        if let (Ok(player_position), Ok(npc_position)) = (
            query_player_position.get_single(),
            query_position.get(dialog_state.npc_entity),
        ) {
            if npc_position
                .position
                .xy()
                .distance(player_position.position.xy())
                > 400.0
            {
                *current_dialog_state = None;
                return;
            }
        }

        egui::Window::new("NPC Dialog")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .open(&mut open)
            .show(egui_context.ctx_mut(), |ui| {
                if let Some(text_style) =
                    ui.style_mut().text_styles.get_mut(&egui::TextStyle::Button)
                {
                    text_style.size = 16.0;
                }
                if let Some(text_style) = ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body)
                {
                    text_style.size = 16.0;
                }

                ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 10.0);
                ui.spacing_mut().button_padding = egui::Vec2::new(5.0, 5.0);
                ui.label(&dialog_state.generated_dialog.message);
                ui.separator();

                for (index, response) in dialog_state.generated_dialog.responses.iter().enumerate()
                {
                    if ui.button(format!("{}. {}", index, response.text)).clicked() {
                        selected_response = Some(index);
                    }
                }
            });

        if !open {
            // User closed the dialog
            *current_dialog_state = None;
            return;
        }

        if let Some(selected_response) = selected_response {
            let click_action_function =
                &dialog_state.generated_dialog.responses[selected_response].action_function;
            if !click_action_function.is_empty() {
                if let Err(error) = dialog_state.lua_vm.call_global_closure(
                    &mut user_context,
                    click_action_function,
                    &[Lua4Value::UserData(
                        dialog_state.event_object_handle.clone(),
                    )],
                ) {
                    log::error!(
                        "Error running conversation click action function {}: {}",
                        click_action_function,
                        error
                    );
                }
            }

            if !dialog_state.generated_dialog.run_menu(
                &mut dialog_state.lua_vm,
                &mut user_context,
                &dialog_state.con_file,
                &dialog_state.event_object_handle,
                &game_data,
                dialog_state.generated_dialog.responses[selected_response].menu_index,
            ) {
                *current_dialog_state = None;
            }
        }
    }
}
