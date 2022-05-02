use bevy::{
    math::{Vec2, Vec3Swizzles},
    prelude::{AssetServer, Assets, Handle, Image, Local, Query, Res, ResMut, Transform, With},
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};

use rose_data::ZoneId;

use crate::{
    components::{PlayerCharacter, Position},
    resources::{CurrentZone, GameData, Icons},
};

const PLAYER_ICON_SIZE: egui::Vec2 = egui::Vec2::new(16.0, 16.0);
const MAP_BLOCK_PIXELS: f32 = 64.0;
const MAP_OUTLINE_PIXELS: f32 = MAP_BLOCK_PIXELS;

#[derive(Default)]
pub struct UiStateMinimap {
    pub zone_id: Option<ZoneId>,
    pub minimap_image: Handle<Image>,
    pub minimap_texture: egui::TextureId,
    pub minimap_image_size: Option<Vec2>,
    pub min_world_pos: Vec2,
    pub max_world_pos: Vec2,
    pub distance_per_pixel: f32,
    pub last_player_position: Vec2,
}

pub fn ui_minimap_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiStateMinimap>,
    player_position: Query<&Position, With<PlayerCharacter>>,
    asset_server: Res<AssetServer>,
    query_camera: Query<&Transform, With<Camera3d>>,
    images: Res<Assets<Image>>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    if current_zone.is_none() {
        return;
    }
    let current_zone = current_zone.unwrap();

    let camera_forward_2d = query_camera.single().forward().xz().normalize_or_zero();
    let camera_angle = -camera_forward_2d.angle_between(Vec2::Y);

    // If zone has changed, reload the minimap image
    if ui_state.zone_id != Some(current_zone.id) {
        ui_state.minimap_image = Default::default();
        ui_state.minimap_texture = Default::default();
        ui_state.minimap_image_size = Default::default();

        if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
            if let Some(minimap_path) = zone_data.minimap_path.as_ref() {
                ui_state.minimap_image = asset_server.load(minimap_path.path());
                ui_state.minimap_texture =
                    egui_context.add_image(ui_state.minimap_image.clone_weak());
            }
        }

        ui_state.zone_id = Some(current_zone.id);
    }

    if ui_state.minimap_image_size.is_none() {
        if let Some(minimap_image) = images.get(&ui_state.minimap_image) {
            let minimap_image_size = minimap_image.size();
            ui_state.minimap_image_size = Some(minimap_image_size);

            if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
                let world_block_size = 16.0 * current_zone.grid_per_patch * current_zone.grid_size;
                let minimap_blocks_x =
                    (minimap_image_size.x - 2.0 * MAP_OUTLINE_PIXELS) / MAP_BLOCK_PIXELS;
                let minimap_blocks_y =
                    (minimap_image_size.y - 2.0 * MAP_OUTLINE_PIXELS) / MAP_BLOCK_PIXELS;

                let min_pos_x = zone_data.minimap_start_x as f32 * world_block_size;
                let min_pos_y = (64.0 - zone_data.minimap_start_y as f32 + 1.0) * world_block_size;

                let max_pos_x = min_pos_x + minimap_blocks_x * world_block_size;
                let max_pos_y = min_pos_y - minimap_blocks_y * world_block_size;

                ui_state.min_world_pos = Vec2::new(min_pos_x, min_pos_y);
                ui_state.max_world_pos = Vec2::new(max_pos_x, max_pos_y);
                ui_state.distance_per_pixel = world_block_size / MAP_BLOCK_PIXELS;
            }
        }
    }

    let player_position = player_position.get_single().ok();
    let player_position_changed = if let Some(player_position) = player_position {
        if ui_state.minimap_image_size.is_some()
            && ui_state.last_player_position != player_position.position.xy()
        {
            ui_state.last_player_position = player_position.position.xy();
            true
        } else {
            false
        }
    } else {
        false
    };

    egui::Window::new(
        game_data
            .zone_list
            .get_zone(current_zone.id)
            .map(|x| x.name.as_str())
            .unwrap_or("???"),
    )
    .id(egui::Id::new("minimap"))
    .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
    .default_size([225.0, 225.0])
    .show(egui_context.ctx_mut(), |ui| {
        let minimap_visible_size = ui.available_size();
        let minimap_min_rect = ui.min_rect();

        if let Some(minimap_image_size) = ui_state.minimap_image_size {
            let mut scroll_area = egui::ScrollArea::both();

            let minimap_player_pos = if let Some(player_position) = player_position {
                let minimap_player_x = minimap_min_rect.left()
                    + MAP_OUTLINE_PIXELS
                    + f32::max(
                        0.0,
                        (player_position.position.x - ui_state.min_world_pos.x)
                            / ui_state.distance_per_pixel,
                    );
                let minimap_player_y = minimap_min_rect.top()
                    + MAP_OUTLINE_PIXELS
                    + f32::max(
                        0.0,
                        (ui_state.min_world_pos.y - player_position.position.y)
                            / ui_state.distance_per_pixel,
                    );
                Some(egui::pos2(minimap_player_x, minimap_player_y))
            } else {
                None
            };

            if player_position_changed {
                if let Some(target_center) = minimap_player_pos {
                    let scroll_max_x = f32::max(0.0, minimap_image_size.x - minimap_visible_size.x);
                    let scroll_max_y = f32::max(0.0, minimap_image_size.y - minimap_visible_size.y);
                    let visible_center = minimap_min_rect.left_top() + (minimap_visible_size / 2.0);
                    let mut scroll_offset = target_center - visible_center;
                    scroll_offset.x = f32::min(scroll_max_x, f32::max(0.0, scroll_offset.x));
                    scroll_offset.y = f32::min(scroll_max_y, f32::max(0.0, scroll_offset.y));
                    scroll_area = scroll_area.scroll_offset(scroll_offset);
                }
            }

            scroll_area.show_viewport(ui, |ui, visible_rect| {
                let scroll_offset = visible_rect.left_top().to_vec2();

                ui.image(
                    ui_state.minimap_texture,
                    egui::Vec2::new(minimap_image_size.x, minimap_image_size.y),
                );

                // Draw player position arrow texture on a rotated rectangle to face camera position
                if let Some(minimap_player_pos) = minimap_player_pos {
                    let minimap_player_pos = minimap_player_pos - scroll_offset;
                    let widget_rect = egui::Rect::from_min_size(
                        minimap_player_pos - PLAYER_ICON_SIZE / 2.0,
                        PLAYER_ICON_SIZE,
                    );

                    ui.allocate_ui_at_rect(widget_rect, |ui| {
                        ui.centered_and_justified(|ui| {
                            let (rect, response) =
                                ui.allocate_exact_size(PLAYER_ICON_SIZE, egui::Sense::hover());

                            // Calculate rotated rectangle from camera angle
                            let sin_a = camera_angle.sin();
                            let cos_a = camera_angle.cos();

                            let mut corners = [
                                [-PLAYER_ICON_SIZE.x / 2.0, -PLAYER_ICON_SIZE.y / 2.0],
                                [PLAYER_ICON_SIZE.x / 2.0, -PLAYER_ICON_SIZE.y / 2.0],
                                [-PLAYER_ICON_SIZE.x / 2.0, PLAYER_ICON_SIZE.y / 2.0],
                                [PLAYER_ICON_SIZE.x / 2.0, PLAYER_ICON_SIZE.y / 2.0],
                            ];

                            for corner in corners.iter_mut() {
                                let rotated_x = corner[0] * cos_a - corner[1] * sin_a;
                                let rotated_y = corner[0] * sin_a + corner[1] * cos_a;
                                *corner = [rotated_x, rotated_y];
                            }

                            if ui.is_rect_visible(rect) {
                                let mut mesh =
                                    egui::Mesh::with_texture(icons.minimap_player_icon.1);
                                let uv = egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                );

                                let color = egui::Color32::WHITE;
                                let idx = mesh.vertices.len() as u32;
                                mesh.add_triangle(idx, idx + 1, idx + 2);
                                mesh.add_triangle(idx + 2, idx + 1, idx + 3);

                                mesh.vertices.push(egui::epaint::Vertex {
                                    pos: minimap_player_pos + egui::Vec2::from(corners[0]),
                                    uv: uv.left_top(),
                                    color,
                                });
                                mesh.vertices.push(egui::epaint::Vertex {
                                    pos: minimap_player_pos + egui::Vec2::from(corners[1]),
                                    uv: uv.right_top(),
                                    color,
                                });
                                mesh.vertices.push(egui::epaint::Vertex {
                                    pos: minimap_player_pos + egui::Vec2::from(corners[2]),
                                    uv: uv.left_bottom(),
                                    color,
                                });
                                mesh.vertices.push(egui::epaint::Vertex {
                                    pos: minimap_player_pos + egui::Vec2::from(corners[3]),
                                    uv: uv.right_bottom(),
                                    color,
                                });

                                ui.painter().add(egui::Shape::mesh(mesh));
                            }

                            response
                        })
                        .inner
                    });
                }
            });
        }
    });
}
