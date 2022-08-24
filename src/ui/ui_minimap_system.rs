use std::sync::Arc;

use bevy::{
    math::{Vec2, Vec3Swizzles},
    prelude::{
        AssetServer, Assets, Camera3d, Handle, Image, Local, Query, Res, ResMut, Transform, With,
    },
};
use bevy_egui::{egui, EguiContext};

use rose_data::ZoneId;

use crate::{
    components::{PlayerCharacter, Position},
    resources::{CurrentZone, GameData, UiResources},
    ui::widgets::{DataBindings, Dialog, Widget},
    zone_loader::ZoneLoaderAsset,
};

const MAP_BLOCK_PIXELS: f32 = 64.0;
const MAP_OUTLINE_PIXELS: f32 = MAP_BLOCK_PIXELS;

const ZONE_NAME_WIDTH: f32 = 102.0;
const ZONE_NAME_EXPANDED_WIDTH: f32 = 172.0;

const IID_PANE_BIG: i32 = 50;
// const IID_CAPTION_BIG: i32 = 51;
const IID_BTN_NORMAL: i32 = 52;
const IID_BTN_MINIMIZE_BIG: i32 = 53;
const IID_PANE_BIG_CHILDPANE: i32 = 60;
const IID_PANE_SMALL: i32 = 100;
// const IID_CAPTION_SMALL: i32 = 101;
const IID_BTN_EXPAND: i32 = 102;
const IID_BTN_MINIMIZE_SMALL: i32 = 103;
const IID_PANE_SMALL_CHILDPANE: i32 = 110;

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
    pub is_expanded: bool,
    pub is_minimised: bool,
    pub scroll: Vec2,
    pub zone_name_text_galley: Option<Arc<egui::Galley>>,
    pub zone_name_text_expanded_galley: Option<Arc<egui::Galley>>,
}

fn generate_text_galley(
    ctx: &egui::Context,
    width: f32,
    height: f32,
    text: String,
) -> Arc<egui::Galley> {
    let style = ctx.style();
    let text_format = egui::text::TextFormat {
        font_id: egui::FontSelection::Default.resolve(&style),
        color: egui::Color32::WHITE,
        background: egui::Color32::TRANSPARENT,
        italics: false,
        underline: egui::Stroke::none(),
        strikethrough: egui::Stroke::none(),
        valign: egui::Align::Center,
    };

    let mut text_job = egui::text::LayoutJob::single_section(text, text_format);
    text_job.first_row_min_height = height;
    text_job.wrap.max_width = width;
    text_job.wrap.max_rows = 1;
    text_job.wrap.break_anywhere = true;

    ctx.fonts().layout_job(text_job)
}

pub fn ui_minimap_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiStateMinimap>,
    player_position: Query<&Position, With<PlayerCharacter>>,
    asset_server: Res<AssetServer>,
    query_camera: Query<&Transform, With<Camera3d>>,
    images: Res<Assets<Image>>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_minimap) {
        dialog
    } else {
        return;
    };

    if current_zone.is_none() {
        return;
    }
    let current_zone = current_zone.unwrap();
    let current_zone_data = zone_loader_assets.get(&current_zone.handle).unwrap();

    let camera_forward_2d = query_camera.single().forward().xz().normalize_or_zero();
    let camera_angle = -camera_forward_2d.angle_between(Vec2::Y);

    // If zone has changed, reload the minimap image
    if ui_state.zone_id != Some(current_zone.id) {
        ui_state.minimap_image = Default::default();
        ui_state.minimap_texture = Default::default();
        ui_state.minimap_image_size = Default::default();

        let zone_name = if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
            if let Some(minimap_path) = zone_data.minimap_path.as_ref() {
                ui_state.minimap_image = asset_server.load(minimap_path.path());
                ui_state.minimap_texture =
                    egui_context.add_image(ui_state.minimap_image.clone_weak());
            }

            zone_data.name
        } else {
            "???"
        };

        let ctx = egui_context.ctx_mut();
        ui_state.zone_name_text_galley = Some(generate_text_galley(
            ctx,
            ZONE_NAME_WIDTH,
            16.0,
            zone_name.to_string(),
        ));
        ui_state.zone_name_text_expanded_galley = Some(generate_text_galley(
            ctx,
            ZONE_NAME_EXPANDED_WIDTH,
            16.0,
            zone_name.to_string(),
        ));
        ui_state.zone_id = Some(current_zone.id);
        ui_state.last_player_position = Vec2::default();
    }

    if ui_state.minimap_image_size.is_none() {
        if let Some(minimap_image) = images.get(&ui_state.minimap_image) {
            let minimap_image_size = minimap_image.size();
            ui_state.minimap_image_size = Some(minimap_image_size);

            if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
                let world_block_size =
                    16.0 * current_zone_data.zon.grid_per_patch * current_zone_data.zon.grid_size;
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
            && ui_state.last_player_position != player_position.xy()
        {
            ui_state.last_player_position = player_position.xy();
            true
        } else {
            false
        }
    } else {
        false
    };

    let (dialog_width, dialog_height) = if ui_state.is_expanded {
        if let Some(Widget::Pane(pane)) = dialog.get_widget(IID_PANE_BIG) {
            (pane.width, pane.height)
        } else {
            (dialog.width, dialog.height)
        }
    } else if let Some(Widget::Pane(pane)) = dialog.get_widget(IID_PANE_SMALL) {
        (pane.width, pane.height)
    } else {
        (dialog.width, dialog.height)
    };

    let mut response_expand_button = None;
    let mut response_shrink_button = None;
    let mut response_big_minimise_button = None;
    let mut response_small_minimise_button = None;
    let minimised = ui_state.minimap_image_size.is_none() || ui_state.is_minimised;

    egui::Window::new("Minimap")
        .anchor(egui::Align2::RIGHT_TOP, [0.0, 0.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog_width)
        .default_height(dialog_height)
        .show(egui_context.ctx_mut(), |ui| {
            let minimap_size = Vec2::new(dialog_width - 2.0, dialog_height - 22.0);
            let image_size = ui_state.minimap_image_size.unwrap_or(minimap_size);
            let minimap_rect = egui::Rect::from_min_size(
                ui.min_rect().min + egui::vec2(1.0, 21.0),
                egui::vec2(minimap_size.x, minimap_size.y),
            );
            let minimap_player_pos = if let Some(player_position) = player_position {
                let minimap_player_x = MAP_OUTLINE_PIXELS
                    + f32::max(
                        0.0,
                        (player_position.x - ui_state.min_world_pos.x)
                            / ui_state.distance_per_pixel,
                    );
                let minimap_player_y = MAP_OUTLINE_PIXELS
                    + f32::max(
                        0.0,
                        (ui_state.min_world_pos.y - player_position.y)
                            / ui_state.distance_per_pixel,
                    );
                Some(Vec2::new(minimap_player_x, minimap_player_y))
            } else {
                None
            };

            if !minimised {
                let response = ui.allocate_rect(minimap_rect, egui::Sense::click_and_drag());

                if response.dragged() {
                    let delta = ui.input().pointer.delta();
                    ui_state.scroll.x -= delta.x;
                    ui_state.scroll.y -= delta.y;
                } else if player_position_changed {
                    if let Some(target_center) = minimap_player_pos {
                        let visible_center = ui_state.scroll + (minimap_size / 2.0);
                        ui_state.scroll += target_center - visible_center;
                    }
                }

                ui_state.scroll.x = ui_state
                    .scroll
                    .x
                    .clamp(0.0, (image_size.x - minimap_size.x).max(0.0));
                ui_state.scroll.y = ui_state
                    .scroll
                    .y
                    .clamp(0.0, (image_size.y - minimap_size.y).max(0.0));

                let minimap_uv = egui::Rect::from_min_max(
                    egui::pos2(
                        ui_state.scroll.x / image_size.x,
                        ui_state.scroll.y / image_size.y,
                    ),
                    egui::pos2(
                        (ui_state.scroll.x + minimap_size.x) / image_size.x,
                        (ui_state.scroll.y + minimap_size.y) / image_size.y,
                    ),
                );

                if ui.is_rect_visible(minimap_rect) {
                    let mut mesh = egui::epaint::Mesh::with_texture(ui_state.minimap_texture);
                    mesh.add_rect_with_uv(minimap_rect, minimap_uv, egui::Color32::WHITE);
                    ui.painter().add(egui::epaint::Shape::mesh(mesh));
                }
            }

            dialog.draw(
                ui,
                DataBindings {
                    response: &mut [
                        (IID_BTN_EXPAND, &mut response_expand_button),
                        (IID_BTN_NORMAL, &mut response_shrink_button),
                        (IID_BTN_MINIMIZE_BIG, &mut response_big_minimise_button),
                        (IID_BTN_MINIMIZE_SMALL, &mut response_small_minimise_button),
                    ],
                    visible: &mut [
                        (IID_PANE_SMALL, !ui_state.is_expanded),
                        (IID_PANE_BIG, ui_state.is_expanded),
                        (IID_PANE_BIG_CHILDPANE, !ui_state.is_minimised),
                        (IID_PANE_SMALL_CHILDPANE, !ui_state.is_minimised),
                    ],
                    ..Default::default()
                },
                |ui, _bindings| {
                    let zone_name_width = if ui_state.is_expanded {
                        ZONE_NAME_EXPANDED_WIDTH
                    } else {
                        ZONE_NAME_WIDTH
                    };
                    let zone_name_rect = egui::Rect::from_min_size(
                        egui::pos2(5.0, 2.0),
                        egui::vec2(zone_name_width, 16.0),
                    )
                    .translate(ui.min_rect().min.to_vec2());

                    ui.allocate_ui_at_rect(zone_name_rect, |ui| {
                        let galley = if ui_state.is_expanded {
                            ui_state.zone_name_text_expanded_galley.as_ref()
                        } else {
                            ui_state.zone_name_text_galley.as_ref()
                        };

                        if let Some(galley) = galley {
                            ui.horizontal_top(|ui| ui.label(galley.clone()));
                        }
                    });
                },
            );

            if !minimised {
                // Draw player position arrow texture on a rotated rectangle to face camera position
                if let Some(minimap_player_pos) = minimap_player_pos {
                    let minimap_player_sprite = ui_resources.get_minimap_player_sprite().unwrap();
                    let player_icon_size =
                        Vec2::new(minimap_player_sprite.width, minimap_player_sprite.height);
                    let minimap_player_pos = Vec2::new(minimap_rect.min.x, minimap_rect.min.y)
                        + minimap_player_pos
                        - ui_state.scroll;
                    let widget_rect = egui::Rect::from_min_size(
                        (minimap_player_pos - player_icon_size / 2.0)
                            .to_array()
                            .into(),
                        player_icon_size.to_array().into(),
                    );

                    if minimap_rect.contains_rect(widget_rect) {
                        ui.allocate_ui_at_rect(widget_rect, |ui| {
                            ui.centered_and_justified(|ui| {
                                let (rect, response) = ui.allocate_exact_size(
                                    player_icon_size.to_array().into(),
                                    egui::Sense::hover(),
                                );

                                // Calculate rotated rectangle from camera angle
                                let sin_a = camera_angle.sin();
                                let cos_a = camera_angle.cos();

                                let mut corners = [
                                    [-player_icon_size.x / 2.0, -player_icon_size.y / 2.0],
                                    [player_icon_size.x / 2.0, -player_icon_size.y / 2.0],
                                    [-player_icon_size.x / 2.0, player_icon_size.y / 2.0],
                                    [player_icon_size.x / 2.0, player_icon_size.y / 2.0],
                                ];

                                for corner in corners.iter_mut() {
                                    let rotated_x = corner[0] * cos_a - corner[1] * sin_a;
                                    let rotated_y = corner[0] * sin_a + corner[1] * cos_a;
                                    *corner = [rotated_x, rotated_y];
                                }

                                if ui.is_rect_visible(rect) {
                                    let mut mesh =
                                        egui::Mesh::with_texture(minimap_player_sprite.texture_id);
                                    let uv = minimap_player_sprite.uv;

                                    let color = egui::Color32::WHITE;
                                    let idx = mesh.vertices.len() as u32;
                                    mesh.add_triangle(idx, idx + 1, idx + 2);
                                    mesh.add_triangle(idx + 2, idx + 1, idx + 3);

                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos + Vec2::from(corners[0]))
                                            .to_array()
                                            .into(),
                                        uv: uv.left_top(),
                                        color,
                                    });
                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos + Vec2::from(corners[1]))
                                            .to_array()
                                            .into(),
                                        uv: uv.right_top(),
                                        color,
                                    });
                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos + Vec2::from(corners[2]))
                                            .to_array()
                                            .into(),
                                        uv: uv.left_bottom(),
                                        color,
                                    });
                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos + Vec2::from(corners[3]))
                                            .to_array()
                                            .into(),
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
                }
            }
        });

    if response_expand_button.map_or(false, |r| r.clicked()) {
        ui_state.is_expanded = true;
    }

    if response_shrink_button.map_or(false, |r| r.clicked()) {
        ui_state.is_expanded = false;
    }

    if response_big_minimise_button.map_or(false, |r| r.clicked())
        || response_small_minimise_button.map_or(false, |r| r.clicked())
    {
        ui_state.is_minimised = !ui_state.is_minimised;
    }
}
