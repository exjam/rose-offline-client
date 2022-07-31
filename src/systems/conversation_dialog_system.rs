use std::sync::Arc;

use bevy::{
    math::Vec3Swizzles,
    prelude::{Assets, Entity, EventReader, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};
use rose_file_readers::{ConFile, ConMessageType};

use crate::{
    components::{ClientEntityName, PlayerCharacter, Position},
    events::ConversationDialogEvent,
    resources::{GameData, UiResources, UiSprite},
    scripting::{
        lua4::{Lua4Function, Lua4VM, Lua4VMError, Lua4VMRustClosures, Lua4Value},
        LuaGameConstants, LuaGameFunctions, LuaQuestFunctions, LuaUserValueEntity,
        ScriptFunctionContext, ScriptFunctionResources,
    },
    ui::{widgets::Dialog, DataBindings, DialogInstance},
    VfsResource,
};

pub struct GeneratedDialogResponse {
    pub text: egui::text::LayoutJob,
    pub galley: Option<Arc<egui::text::Galley>>,
    pub action_function: String,
    pub menu_index: i32,
}

#[derive(Default)]
pub struct GeneratedDialog {
    pub message: egui::text::LayoutJob,
    pub responses: Vec<GeneratedDialogResponse>,
}

pub struct ConversationDialogState {
    pub owner_entity: Option<Entity>,
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
    owner_entity: Option<Entity>,
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
        owner_entity,
        con_file,
        event_object_handle: Arc::new(LuaUserValueEntity { owner_entity }),
        generated_dialog: Default::default(),
        lua_vm,
    })
}

fn parse_message(message: &str, user_context: &LuaVMContext) -> String {
    let mut string = String::with_capacity(message.len());

    let mut remaining = message;
    while let Some(template_start) = remaining.find(|c| c == '<') {
        let (before_template, template) = remaining.split_at(template_start);

        let template_end = template.find(|c| c == '>');
        if template_end.is_none() {
            return string;
        }
        let template_end = template_end.unwrap();
        let (template, after_template) = template.split_at(template_end + 1);

        string += before_template;
        string += match template {
            "<NAME>" => user_context
                .function_context
                .query_player
                .get_single()
                .map(|player| player.character_info.name.clone())
                .ok(),
            "<LEVEL>" => user_context
                .function_context
                .query_player
                .get_single()
                .map(|player| format!("{}", player.level.level))
                .ok(),
            _ => None,
        }
        .unwrap_or_else(|| template.to_string())
        .as_str();
        remaining = after_template;
    }

    string += remaining;
    string
}

fn message_layout_job(response_number: Option<usize>, message: &str) -> egui::text::LayoutJob {
    let default_text_color = egui::Color32::BLACK;
    let mut remaining = message;
    let mut job = egui::text::LayoutJob::default();
    let mut current_text_format = egui::text::TextFormat {
        color: default_text_color,
        ..Default::default()
    };
    job.wrap.max_width = 300.0;

    if let Some(response_number) = response_number {
        job.append(
            &format!("{}. ", response_number + 1),
            0.0,
            current_text_format.clone(),
        );
    }

    while let Some(tag_start) = remaining.find('{') {
        let (before_tag, tag) = remaining.split_at(tag_start);
        let tag_end = tag.find('}');
        if tag_end.is_none() {
            break;
        }
        let tag_end = tag_end.unwrap();
        let (tag, after_tag) = tag.split_at(tag_end + 1);

        let tag_lower = tag.to_lowercase();
        match tag_lower.as_str() {
            "{br}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                job.append("\n", 0.0, current_text_format.clone());
            }
            "{b}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                current_text_format.italics = true;
            }
            "{/b}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                current_text_format.italics = false;
            }
            "{/fc}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                current_text_format.color = default_text_color;
            }
            tag if tag.starts_with("{fc=") => {
                let len = tag.len();
                let index_str = &tag[4..len - 1];
                if let Ok(color_index) = index_str.parse::<i32>() {
                    job.append(before_tag, 0.0, current_text_format.clone());
                    current_text_format.color = match color_index {
                        0 => egui::Color32::from_rgb(0, 0, 0),
                        1 => egui::Color32::from_rgb(0x80, 0, 0),
                        2 => egui::Color32::from_rgb(0, 0x80, 0),
                        3 => egui::Color32::from_rgb(0, 0, 0x80),
                        4 => egui::Color32::from_rgb(0x80, 0x80, 0),
                        5 => egui::Color32::from_rgb(0x80, 0, 0x80),
                        6 => egui::Color32::from_rgb(0, 0x80, 0x80),
                        7 => egui::Color32::from_rgb(0x80, 0x80, 0x80),
                        8 => egui::Color32::from_rgb(0xC0, 0xC0, 0xC0),
                        9 => egui::Color32::from_rgb(0xC0, 0xDC, 0xC0),
                        10 => egui::Color32::from_rgb(0xC0, 0xC0, 0xDC),
                        11 => egui::Color32::from_rgb(0xA6, 0xCA, 0xF0),
                        12 => egui::Color32::from_rgb(0xFF, 0, 0),
                        13 => egui::Color32::from_rgb(0, 0xFF, 0),
                        14 => egui::Color32::from_rgb(0, 0, 0xFF),
                        15 => egui::Color32::from_rgb(0xFF, 0xFF, 0),
                        16 => egui::Color32::from_rgb(0, 0xFF, 0xFF),
                        17 => egui::Color32::from_rgb(0xFF, 0xFB, 0xF0),
                        18 => egui::Color32::from_rgb(0xFF, 0xFF, 0xFF),
                        _ => default_text_color,
                    };
                }
            }
            _ => {}
        }

        remaining = after_tag;
    }

    if !remaining.is_empty() {
        job.append(remaining, 0.0, current_text_format);
    }

    job
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
                        text: message_layout_job(
                            Some(self.responses.len()),
                            game_data
                                .ltb_event
                                .get_string(message.string_id as usize, 2)
                                .map(|message| parse_message(&message, user_context))
                                .unwrap_or_else(|| "???".into())
                                .as_str(),
                        ),
                        galley: None,
                        action_function: message.action_function.clone(),
                        menu_index: message.message_value,
                    });
                }
                ConMessageType::NextMessage | ConMessageType::ShowMessage => {
                    self.message = message_layout_job(
                        None,
                        game_data
                            .ltb_event
                            .get_string(message.string_id as usize, 2)
                            .map(|message| parse_message(&message, user_context))
                            .unwrap_or_else(|| "???".into())
                            .as_str(),
                    );
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

struct UiConversationDialogSprites {
    message_top: UiSprite,
    message_middle: UiSprite,
    message_bottom: UiSprite,
    answer_top: UiSprite,
    answer_middle: UiSprite,
    answer_bottom: UiSprite,
}

pub struct UiConversationDialogState {
    dialog_instance: DialogInstance,
    sprites: Option<UiConversationDialogSprites>,
}

impl Default for UiConversationDialogState {
    fn default() -> Self {
        Self {
            dialog_instance: DialogInstance::new("DLGDIALOG.XML"),
            sprites: None,
        }
    }
}

pub fn conversation_dialog_system(
    mut current_dialog_state: Local<Option<ConversationDialogState>>,
    mut egui_context: ResMut<EguiContext>,
    mut conversation_dialog_events: EventReader<ConversationDialogEvent>,
    mut lua_function_context: ScriptFunctionContext,
    mut ui_state: Local<UiConversationDialogState>,
    script_function_resources: ScriptFunctionResources,
    query_player_position: Query<&Position, With<PlayerCharacter>>,
    query_position: Query<&Position>,
    query_name: Query<&ClientEntityName>,
    lua_game_constants: Res<LuaGameConstants>,
    lua_game_functions: Res<LuaGameFunctions>,
    lua_quest_functions: Res<LuaQuestFunctions>,
    game_data: Res<GameData>,
    vfs_resource: Res<VfsResource>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = ui_state
        .dialog_instance
        .get_mut(&dialog_assets, &ui_resources)
    {
        dialog
    } else {
        return;
    };
    let dialog_sprites = ui_state
        .sprites
        .get_or_insert_with(|| UiConversationDialogSprites {
            message_top: ui_resources
                .get_sprite(0, "UI13_NPC_SCRIPT_IMAGE_TOP")
                .unwrap(),
            message_middle: ui_resources
                .get_sprite(0, "UI13_NPC_SCRIPT_IMAGE_MIDDLE")
                .unwrap(),
            message_bottom: ui_resources
                .get_sprite(0, "UI13_NPC_SCRIPT_IMAGE_BOTTOM")
                .unwrap(),
            answer_top: ui_resources
                .get_sprite(0, "UI13_NPC_SCRIPT_ANSWER_TOP")
                .unwrap(),
            answer_middle: ui_resources
                .get_sprite(0, "UI13_NPC_SCRIPT_ANSWER_MIDDLE")
                .unwrap(),
            answer_bottom: ui_resources
                .get_sprite(0, "UI13_NPC_SCRIPT_ANSWER_BOTTOM")
                .unwrap(),
        });

    let mut user_context = LuaVMContext {
        function_context: &mut lua_function_context,
        function_resources: &script_function_resources,
        game_constants: &lua_game_constants,
        game_functions: &lua_game_functions,
        quest_functions: &lua_quest_functions,
    };

    for event in conversation_dialog_events.iter() {
        let (owner_entity, con_file_path) = match event {
            ConversationDialogEvent::OpenNpcDialog(npc_entity, con_file_path) => {
                (Some(*npc_entity), con_file_path)
            }
            ConversationDialogEvent::OpenEventDialog(con_file_path) => (None, con_file_path),
        };
        *current_dialog_state = None;

        if let Some(mut next_dialog_state) = vfs_resource
            .vfs
            .read_file::<ConFile, _>(con_file_path)
            .ok()
            .and_then(|con_file| {
                create_conversation_dialog(con_file, &mut user_context, owner_entity)
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
        if let (Ok(player_position), Some(npc_position)) = (
            query_player_position.get_single(),
            dialog_state
                .owner_entity
                .and_then(|entity| query_position.get(entity).ok()),
        ) {
            if npc_position.position.xy().distance(player_position.xy()) > 400.0 {
                *current_dialog_state = None;
                return;
            }
        }

        let fonts = egui_context.ctx_mut().fonts();
        let message_galley = fonts.layout_job(dialog_state.generated_dialog.message.clone());

        let message_size = message_galley.size();
        let num_message_middle = ((message_size.y + dialog_sprites.message_middle.height - 1.0)
            / dialog_sprites.message_middle.height) as usize;

        let mut response_size_y =
            (dialog_state.generated_dialog.responses.len().max(1) - 1) as f32 * 5.0;
        for response in dialog_state.generated_dialog.responses.iter_mut() {
            let galley = fonts.layout_job(response.text.clone());
            response_size_y += galley.size().y;
            response.galley = Some(galley);
        }
        let num_response_middles = ((response_size_y + dialog_sprites.answer_middle.height - 1.0)
            / dialog_sprites.answer_middle.height) as usize;

        std::mem::drop(fonts);

        let dialog_height = dialog.height
            + dialog_sprites.message_top.height
            + num_message_middle as f32 * dialog_sprites.message_middle.height
            + dialog_sprites.message_bottom.height
            + dialog_sprites.answer_top.height
            + num_response_middles as f32 * dialog_sprites.answer_middle.height
            + dialog_sprites.answer_bottom.height;

        let title = dialog_state
            .owner_entity
            .and_then(|entity| query_name.get(entity).ok())
            .map(|name| name.as_str())
            .unwrap_or("Event Dialog");

        let mut response_close_button = None;
        let screen_rect = egui_context.ctx_mut().input().screen_rect();
        let default_x = screen_rect.width() / 2.0 - dialog.width / 2.0;
        let default_y = screen_rect.height() / 2.0 - dialog_height / 2.0;

        egui::Window::new("Conversation Dialog")
            .frame(egui::Frame::none())
            .open(&mut open)
            .title_bar(false)
            .resizable(false)
            .min_width(dialog.width)
            .min_height(dialog_height)
            .default_pos([default_x, default_y])
            .show(egui_context.ctx_mut(), |ui| {
                dialog.draw(
                    ui,
                    DataBindings {
                        response: &mut [(10, &mut response_close_button)],
                        ..Default::default()
                    },
                    |ui, _bindings| {
                        ui.put(
                            egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(350.0, 20.0)),
                            egui::Label::new(
                                egui::RichText::new(title).color(egui::Color32::BLACK),
                            ),
                        );

                        let mut pos = ui.min_rect().min + egui::vec2(0.0, 24.0);
                        dialog_sprites.message_top.draw(ui, pos);
                        pos.y += dialog_sprites.message_top.height - 1.0;

                        let message_rect_min = pos;
                        for _ in 0..num_message_middle {
                            dialog_sprites.message_middle.draw(ui, pos);
                            pos.y += dialog_sprites.message_middle.height - 1.0;
                        }

                        dialog_sprites.message_bottom.draw(ui, pos);
                        pos.y += dialog_sprites.message_bottom.height - 1.0;

                        dialog_sprites.answer_top.draw(ui, pos);
                        pos.y += dialog_sprites.answer_top.height - 1.0;

                        let response_rect_min = pos;
                        for _ in 0..num_response_middles {
                            dialog_sprites.answer_middle.draw(ui, pos);
                            pos.y += dialog_sprites.answer_middle.height - 1.0;
                        }

                        dialog_sprites.answer_bottom.draw(ui, pos);
                        pos.y += dialog_sprites.answer_bottom.height;

                        let message_rect = egui::Rect::from_min_max(
                            message_rect_min + egui::vec2(25.0, 0.0),
                            egui::pos2(message_rect_min.x + 25.0 + 300.0, pos.y),
                        );
                        ui.allocate_ui_at_rect(message_rect, |ui| {
                            ui.horizontal_top(|ui| ui.add(egui::Label::new(message_galley.clone())))
                        });

                        let mut response_rect_min = response_rect_min + egui::vec2(15.0, 5.0);
                        for (index, answer) in
                            dialog_state.generated_dialog.responses.iter().enumerate()
                        {
                            let answer_galley = answer.galley.clone().unwrap();
                            let line_rect =
                                egui::Rect::from_min_size(response_rect_min, answer_galley.size());

                            let response = ui.allocate_rect(line_rect, egui::Sense::click());
                            if ui.is_rect_visible(response.rect) {
                                ui.painter().add(egui::epaint::TextShape {
                                    pos: response.rect.left_top(),
                                    galley: answer_galley,
                                    override_text_color: if response.hovered() {
                                        Some(egui::Color32::BLUE)
                                    } else {
                                        None
                                    },
                                    underline: egui::Stroke::none(),
                                    angle: 0.0,
                                });
                            }

                            if response.clicked() {
                                selected_response = Some(index);
                            }

                            response_rect_min.y += line_rect.height() + 5.0;
                        }
                    },
                );
            });

        if response_close_button.map_or(false, |r| r.clicked()) {
            open = false;
        }

        if !open {
            // User closed the dialog
            *current_dialog_state = None;
            return;
        }

        if selected_response.is_none() {
            let input = egui_context.ctx_mut().input();

            for (index, &key) in [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ]
            .iter()
            .enumerate()
            {
                if input.key_pressed(key) {
                    selected_response = Some(index);
                }
            }
        }

        if let Some(selected_response) = selected_response {
            if let Some(selected_response) = dialog_state
                .generated_dialog
                .responses
                .get(selected_response)
            {
                let click_action_function = &selected_response.action_function;
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
                    selected_response.menu_index,
                ) {
                    *current_dialog_state = None;
                }
            }
        }
    }
}
