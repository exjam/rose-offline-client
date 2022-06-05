use bevy::prelude::{Local, Query, Res, ResMut, Time, With};
use bevy_egui::{egui, EguiContext};
use std::collections::VecDeque;
use std::fmt::Write;

use rose_game_common::components::MoveMode;

use crate::{
    components::{
        Command, CommandCastSkillState, CommandCastSkillTarget, CommandSit, PlayerCharacter,
    },
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugCommandViewer {
    pub current_command: Option<Command>,
    pub current_duration: f32,
    pub command_history: VecDeque<(Command, f32)>,
}

pub fn format_command(ui: &mut egui::Ui, command: &Command) {
    match command {
        Command::Stop => {
            ui.label("Stop");
        }
        Command::Move(command_move) => {
            ui.label(format!(
                "{} to ({}, {})",
                match command_move.move_mode {
                    Some(MoveMode::Run) => "Run",
                    Some(MoveMode::Walk) => "Walk",
                    Some(MoveMode::Drive) => "Drive",
                    None => "Move",
                },
                command_move.destination.x,
                command_move.destination.y
            ));
        }
        Command::Attack(command_attack) => {
            ui.label(format!("Attack {}", command_attack.target.id()));
        }
        Command::Die => {
            ui.label("Die");
        }
        Command::PickupItem(pickup_entity) => {
            ui.label(format!("Pickup {}", pickup_entity.id()));
        }
        Command::Emote(command_emote) => {
            ui.label(format!("Emote {}", command_emote.motion_id.get()));
        }
        Command::Sit(CommandSit::Sit) => {
            ui.label("Sit");
        }
        Command::Sit(CommandSit::Sitting) => {
            ui.label("Sit (Sitting)");
        }
        Command::Sit(CommandSit::Standing) => {
            ui.label("Sit (Standing)");
        }
        Command::CastSkill(command_cast_skill) => {
            let mut label = String::with_capacity(128);
            write!(label, "Cast skill {}", command_cast_skill.skill_id.get()).ok();

            match command_cast_skill.skill_target {
                Some(CommandCastSkillTarget::Entity(skill_target_entity)) => {
                    write!(label, " on {}", skill_target_entity.id()).ok();
                }
                Some(CommandCastSkillTarget::Position(skill_position)) => {
                    write!(label, " at ({}, {})", skill_position.x, skill_position.y).ok();
                }
                None => {
                    write!(label, " on self").ok();
                }
            }

            match command_cast_skill.cast_skill_state {
                CommandCastSkillState::Starting => {
                    write!(label, " (Starting)").ok();
                }
                CommandCastSkillState::Casting => {
                    if command_cast_skill.ready_action {
                        write!(label, " (Casting) (ActionReady)").ok();
                    } else {
                        write!(label, " (Casting)").ok();
                    }
                }
                CommandCastSkillState::CastingRepeat => {
                    if command_cast_skill.ready_action {
                        write!(label, " (CastingRepeat) (ActionReady)").ok();
                    } else {
                        write!(label, " (CastingRepeat)").ok();
                    }
                }
                CommandCastSkillState::Action => {
                    write!(label, " (Action)").ok();
                }
            }

            ui.label(label);
        }
    }
}

pub fn ui_debug_command_viewer_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_command_viewer: Local<UiStateDebugCommandViewer>,
    query_player: Query<&Command, With<PlayerCharacter>>,
    time: Res<Time>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    let ctx = egui_context.ctx_mut();
    egui::Window::new("Command Viewer")
        .resizable(false)
        .open(&mut ui_state_debug_windows.command_viewer_open)
        .show(ctx, |ui| {
            if let Ok(command) = query_player.get_single() {
                if ui_state_debug_command_viewer.current_command.as_ref() == Some(command) {
                    ui_state_debug_command_viewer.current_duration += time.delta_seconds();
                } else {
                    if let Some(last_command) = ui_state_debug_command_viewer.current_command.take()
                    {
                        if ui_state_debug_command_viewer.command_history.len() > 20 {
                            ui_state_debug_command_viewer.command_history.pop_front();
                        }

                        let duration = ui_state_debug_command_viewer.current_duration;
                        ui_state_debug_command_viewer
                            .command_history
                            .push_back((last_command, duration));
                    }

                    ui_state_debug_command_viewer.current_command = Some(command.clone());
                    ui_state_debug_command_viewer.current_duration = 0.0;
                }

                egui::Grid::new("command_view_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        for (previous_command, duration) in
                            ui_state_debug_command_viewer.command_history.iter()
                        {
                            format_command(ui, previous_command);
                            ui.label(format!("{: >2.3}s", duration));
                            ui.end_row();
                        }

                        if let Some(current_command) =
                            ui_state_debug_command_viewer.current_command.as_ref()
                        {
                            format_command(ui, current_command);
                            ui.label(format!(
                                "{: >2.3}s",
                                ui_state_debug_command_viewer.current_duration
                            ));
                            ui.end_row();
                        }
                    });
            }
        });
}
