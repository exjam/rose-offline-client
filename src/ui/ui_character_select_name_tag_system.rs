use bevy::prelude::{Camera, Camera3d, GlobalTransform, Query, Res, ResMut, Vec3, With};
use bevy_egui::{egui, EguiContext};

use crate::resources::{CharacterList, CharacterSelectState, GameData};

pub fn ui_character_select_name_tag_system(
    mut egui_context: ResMut<EguiContext>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    character_list: Option<Res<CharacterList>>,
    character_select_state: Res<CharacterSelectState>,
    game_data: Res<GameData>,
) {
    for (camera, camera_transform) in query_camera.iter() {
        if let CharacterSelectState::CharacterSelect(Some(index)) = *character_select_state {
            if let Some(selected_character) = character_list
                .as_ref()
                .and_then(|character_list| character_list.characters.get(index))
            {
                if let Some(screen_pos) = camera.world_to_viewport(
                    camera_transform,
                    game_data.character_select_positions[index].translation
                        + Vec3::new(0.0, 4.0, 0.0),
                ) {
                    let screen_size = egui_context.ctx_mut().input().screen_rect().size();

                    egui::containers::popup::show_tooltip_at(
                        egui_context.ctx_mut(),
                        egui::Id::new("selected_character_plate"),
                        Some(egui::Pos2::new(
                            screen_pos.x - 30.0,
                            screen_size.y - screen_pos.y,
                        )),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&selected_character.info.name)
                                    .font(egui::FontId::proportional(20.0))
                                    .color(if selected_character.delete_time.is_none() {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::RED
                                    }),
                            );
                            ui.label(format!("Level: {}", selected_character.level.level));
                            ui.label(format!("Job: {}", selected_character.info.job));
                            if let Some(delete_time) = selected_character.delete_time.as_ref() {
                                let duration = delete_time.get_time_until_delete();
                                let seconds = duration.as_secs() % 60;
                                let minutes = (duration.as_secs() / 60) % 60;
                                ui.label(format!("Deleted in {:02}m {:02}s", minutes, seconds));
                            }
                        },
                    );
                }
            }
        }
    }
}
