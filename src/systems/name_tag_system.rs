use std::{num::NonZeroU16, sync::Arc};

use arrayvec::ArrayVec;
use bevy::{
    ecs::query::WorldQuery,
    prelude::{
        Assets, BuildChildren, Color, Commands, ComputedVisibility, DespawnRecursiveExt, Entity,
        EventReader, GlobalTransform, Handle, Image, Local, Query, Res, ResMut, Transform, Vec2,
        Vec3, Visibility, With, Without,
    },
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::ImageSampler,
        view::NoFrustumCulling,
    },
    utils::HashMap,
    window::WindowId,
};
use bevy_egui::{egui, EguiContext};

use rose_game_common::components::{Level, Npc, Team};

use crate::{
    components::{
        ClientEntityName, ModelHeight, NameTag, NameTagEntity, NameTagHealthbarBackground,
        NameTagHealthbarForeground, NameTagName, NameTagTargetMark, NameTagType, PlayerCharacter,
    },
    events::LoadZoneEvent,
    render::WorldUiRect,
    resources::{GameData, NameTagSettings, UiResources, UiSpriteSheetType},
};

const ORDER_HEALTH_BACKGROUND: u8 = 0;
const ORDER_HEALTH_FOREGROUND: u8 = 1;
const ORDER_NAME: u8 = 2;
const ORDER_TARGET_MARK: u8 = 2;
const MAX_NAME_ROWS: usize = 2;

pub struct NameTagData {
    pub image: Handle<Image>,
    pub size: Vec2,
    pub rects: ArrayVec<WorldUiRect, MAX_NAME_ROWS>,
}

pub struct NameTagPendingData {
    pub galley: Arc<egui::Galley>,
    pub colors: ArrayVec<Color, MAX_NAME_ROWS>,
    pub name_tag_type: NameTagType,
}

#[derive(Default)]
pub struct NameTagCache {
    pub cache: HashMap<String, NameTagData>,
    pub pending: HashMap<Entity, NameTagPendingData>,
    pub pixels_per_point: f32,
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    level: &'w Level,
    team: &'w Team,
}

#[derive(WorldQuery)]
pub struct NameTagObjectQuery<'w> {
    entity: Entity,
    name: &'w ClientEntityName,
    model_height: &'w ModelHeight,
    npc: Option<&'w Npc>,
    level: Option<&'w Level>,
    team: Option<&'w Team>,
}

pub fn get_monster_name_tag_color(
    player_level: Option<&Level>,
    monster_level: Option<&Level>,
    monster_team: Option<&Team>,
) -> egui::Color32 {
    let level_diff = player_level.map_or(1, |level| level.level) as i32
        - monster_level.map_or(1, |level| level.level) as i32;

    if monster_team.map_or(false, |team| team.id == Team::DEFAULT_NPC_TEAM_ID) {
        egui::Color32::GREEN
    } else if level_diff <= -23 {
        egui::Color32::from_rgb(224, 149, 255)
    } else if level_diff <= -16 {
        egui::Color32::from_rgb(255, 136, 200)
    } else if level_diff <= -10 {
        egui::Color32::from_rgb(255, 113, 107)
    } else if level_diff <= -4 {
        egui::Color32::from_rgb(255, 166, 107)
    } else if level_diff <= 3 {
        egui::Color32::from_rgb(255, 228, 122)
    } else if level_diff <= 8 {
        egui::Color32::from_rgb(150, 255, 122)
    } else if level_diff <= 14 {
        egui::Color32::from_rgb(137, 243, 255)
    } else if level_diff <= 21 {
        egui::Color32::from_rgb(202, 243, 255)
    } else {
        egui::Color32::from_rgb(217, 217, 217)
    }
}

fn create_pending_nametag(
    name_tag_settings: &NameTagSettings,
    egui_context: &mut EguiContext,
    object: &NameTagObjectQueryItem,
    player: Option<&PlayerQueryItem>,
    name_tag_type: NameTagType,
) -> NameTagPendingData {
    let layout_job = match name_tag_type {
        NameTagType::Character => egui::epaint::text::LayoutJob::single_section(
            object.name.name.clone(),
            egui::TextFormat::simple(
                egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                if object.team.map_or(false, |team| {
                    Some(team.id) != player.map(|player| player.team.id)
                }) {
                    egui::Color32::RED
                } else {
                    egui::Color32::WHITE
                },
            ),
        ),
        NameTagType::Monster => egui::epaint::text::LayoutJob::single_section(
            object.name.name.clone(),
            egui::TextFormat::simple(
                egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                get_monster_name_tag_color(
                    player.map(|player| player.level),
                    object.level,
                    object.team,
                ),
            ),
        ),
        NameTagType::Npc => {
            if let Some((job, name)) = object.name.name.split_once(']') {
                let mut job = job.trim().to_string();
                job.push(']');
                job.push('\n');
                let name = name.trim();

                let mut layout_job = egui::epaint::text::LayoutJob::single_section(
                    job,
                    egui::TextFormat::simple(
                        egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                        egui::Color32::from_rgb(255, 206, 174),
                    ),
                );
                layout_job.append(
                    name,
                    0.0,
                    egui::TextFormat::simple(
                        egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                        egui::Color32::from_rgb(231, 255, 174),
                    ),
                );
                layout_job
            } else {
                egui::epaint::text::LayoutJob::single_section(
                    object.name.name.clone(),
                    egui::TextFormat::simple(
                        egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                        egui::Color32::GREEN,
                    ),
                )
            }
        }
    };

    let colors: ArrayVec<Color, MAX_NAME_ROWS> = layout_job
        .sections
        .iter()
        .map(|x| x.format.color)
        .map(|x| {
            let [r, g, b, _] = x.to_array().map(|c| c as f32 / 255.0);
            Color::rgb_linear(r, g, b)
        })
        .collect();
    let galley = egui_context.ctx_mut().fonts().layout_job(layout_job);

    NameTagPendingData {
        galley,
        colors,
        name_tag_type,
    }
}

fn create_nametag_data(
    egui_context: &mut EguiContext,
    egui_managed_textures: &bevy_egui::EguiManagedTextures,
    images: &mut Assets<Image>,
    pending_data: NameTagPendingData,
) -> Option<NameTagData> {
    let pixels_per_point = egui_context.ctx_mut().pixels_per_point();

    // Calculate the size of name tag text
    let mut max_bounds = Vec2::new(0.0, 0.0);
    let mut row_bounds = Vec::new();
    let mut font_source_textures: ArrayVec<&egui::ColorImage, MAX_NAME_ROWS> = ArrayVec::new();

    for (row_index, row) in pending_data.galley.rows.iter().enumerate() {
        let mut row_min = Vec2::new(10000.0, 10000.0);
        let mut row_max = Vec2::new(0.0, 0.0);

        for glyph in row.glyphs.iter() {
            let glyph_size = Vec2::new(
                glyph.uv_rect.max[0] as f32 - glyph.uv_rect.min[0] as f32,
                glyph.uv_rect.max[1] as f32 - glyph.uv_rect.min[1] as f32,
            );
            let glyph_min = Vec2::new(
                (glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point,
                (glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point,
            );
            let glyph_max = glyph_min + glyph_size;

            row_min = row_min.min(glyph_min);
            row_max = row_max.max(glyph_max);
        }

        let row_start_y = row_index as f32 * 8.0;
        row_min.y += row_start_y;
        row_max.y += row_start_y + 8.0;
        row_max.x += 8.0;

        max_bounds = max_bounds.max(row_max);
        row_bounds.push((row_min, row_max));

        // Get the texture for the font used this row
        let font_texture_id = match row.visuals.mesh.texture_id {
            egui::TextureId::Managed(id) => id,
            egui::TextureId::User(_) => unreachable!(),
        };
        if let Some(managed_texture) = egui_managed_textures
            .0
            .get(&(WindowId::primary(), font_texture_id))
        {
            font_source_textures.push(&managed_texture.color_image);
        } else {
            // Try again next frame
            return None;
        }
    }

    // Allocate texture
    let target_texture_width = (max_bounds.x as u32).next_power_of_two();
    let target_texture_height = (max_bounds.y as u32).next_power_of_two();
    let data_len = (target_texture_width * target_texture_height * 4) as usize;
    let mut data = vec![0; data_len];

    // Copy letters to texture
    for (row_index, row) in pending_data.galley.rows.iter().enumerate() {
        let row_font_texture = &font_source_textures[row_index];
        let src_width = row_font_texture.width();

        for glyph in row.glyphs.iter() {
            let uv_min = glyph.uv_rect.min;
            let uv_max = glyph.uv_rect.max;

            let mut dst_y = ((glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point).round()
                as usize
                + 4
                + row_index * 8;
            for uv_y in uv_min[1]..uv_max[1] {
                let mut dst_x = ((glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point).round()
                    as usize
                    + 4;
                for uv_x in uv_min[0]..uv_max[0] {
                    let pixel = row_font_texture.pixels[uv_y as usize * src_width + uv_x as usize]
                        .to_array();
                    let offset = dst_x * 4 + dst_y * 4 * target_texture_width as usize;
                    unsafe {
                        *data.get_unchecked_mut(offset) = pixel[0].max(*data.get_unchecked(offset));
                        *data.get_unchecked_mut(offset + 1) =
                            pixel[1].max(*data.get_unchecked(offset + 1));
                        *data.get_unchecked_mut(offset + 2) =
                            pixel[2].max(*data.get_unchecked(offset + 2));
                        *data.get_unchecked_mut(offset + 3) =
                            pixel[3].max(*data.get_unchecked(offset + 3));
                    }
                    dst_x += 1;
                }
                dst_y += 1;
            }
        }
    }

    // Apply outline to text
    let mut outlined_data = data.clone();
    unsafe {
        for y in 2..max_bounds.y as u32 - 2 {
            for x in 2..max_bounds.x as u32 - 2 {
                let px_alpha = |x: u32, y: u32| {
                    let offset = (x * 4 + y * 4 * target_texture_width) as usize;
                    *data.get_unchecked(offset + 3) as u32
                };

                let mut alpha = 0u32;
                alpha += px_alpha(x, y - 2);
                alpha += px_alpha(x, y - 1);
                alpha += px_alpha(x, y + 1);
                alpha += px_alpha(x, y + 2);

                alpha += px_alpha(x - 2, y);
                alpha += px_alpha(x - 1, y);
                alpha += px_alpha(x + 1, y);
                alpha += px_alpha(x + 2, y);

                alpha += px_alpha(x - 1, y - 1);
                alpha += px_alpha(x - 1, y + 1);
                alpha += px_alpha(x + 1, y - 1);
                alpha += px_alpha(x + 1, y + 1);
                alpha = alpha.min(255);

                let offset = (x * 4 + y * 4 * target_texture_width) as usize;
                *outlined_data.get_unchecked_mut(offset + 3) = alpha as u8;
            }
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: target_texture_width,
            height: target_texture_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        outlined_data,
        TextureFormat::Rgba8Unorm,
    );
    image.sampler_descriptor = ImageSampler::Descriptor(ImageSampler::nearest_descriptor());
    let image = images.add(image);

    let mut rects: ArrayVec<WorldUiRect, 2> = ArrayVec::new();
    let mut row_offset_y = max_bounds.y - 8.0 * (pending_data.colors.len() - 1) as f32;

    if matches!(pending_data.name_tag_type, NameTagType::Monster) {
        // Give some space for monster health bar under name
        row_offset_y += 15.0;
    }

    // Create WorldUiRect for the outlined text
    for (row_index, row_color) in pending_data.colors.iter().enumerate() {
        let (row_bounds_min, row_bounds_max) = row_bounds[row_index];
        let row_size = row_bounds_max - row_bounds_min;
        let uv_x0 = row_bounds_min.x / target_texture_width as f32;
        let uv_x1 = row_bounds_max.x / target_texture_width as f32;
        let uv_y0 = row_bounds_min.y / target_texture_height as f32;
        let uv_y1 = row_bounds_max.y / target_texture_height as f32;

        rects.push(WorldUiRect {
            screen_offset: Vec2::new(-row_size.x / 2.0, row_offset_y - row_size.y),
            screen_size: row_size,
            image: image.clone_weak(),
            uv_min: Vec2::new(uv_x0, uv_y0),
            uv_max: Vec2::new(uv_x1, uv_y1),
            color: *row_color,
            order: ORDER_NAME,
        });
        row_offset_y -= row_size.y - 8.0;
    }

    Some(NameTagData {
        image,
        size: max_bounds,
        rects,
    })
}

pub fn name_tag_system(
    mut commands: Commands,
    mut name_tag_cache: Local<NameTagCache>,
    query_add: Query<NameTagObjectQuery, Without<NameTagEntity>>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_nametags: Query<(Entity, &NameTagEntity)>,
    egui_managed_textures: Res<bevy_egui::EguiManagedTextures>,
    mut egui_context: ResMut<EguiContext>,
    mut images: ResMut<Assets<Image>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    name_tag_settings: Res<NameTagSettings>,
    mut load_zone_events: EventReader<LoadZoneEvent>,
) {
    let player = query_player.get_single().ok();
    let pixels_per_point = egui_context.ctx_mut().pixels_per_point();

    if load_zone_events.iter().last().is_some()
        || pixels_per_point != name_tag_cache.pixels_per_point
    {
        // When the zone changes, we flush all cached name tag textures to avoid leaking
        // If pixels_per_point has changed then we need to regenerate name tags using new DPI
        for (entity, name_tag_entity) in query_nametags.iter() {
            commands.entity(entity).remove::<NameTagEntity>();
            commands.entity(name_tag_entity.0).despawn_recursive();
        }

        name_tag_cache.cache.clear();
        name_tag_cache.pending.clear();
        name_tag_cache.pixels_per_point = pixels_per_point;
        return;
    }

    for object in query_add.iter() {
        let name_tag_type = if let Some(npc) = object.npc {
            if object
                .team
                .map_or(false, |team| team.id == Team::DEFAULT_NPC_TEAM_ID)
                || game_data
                    .npcs
                    .get_npc(npc.id)
                    .map_or(false, |npc| npc.npc_type_index == NonZeroU16::new(999))
            {
                NameTagType::Npc
            } else {
                NameTagType::Monster
            }
        } else {
            NameTagType::Character
        };

        let name_tag_data = if let Some(name_tag_data) = name_tag_cache.cache.get(&object.name.name)
        {
            name_tag_data
        } else if let Some(pending_name_tag_data) = name_tag_cache.pending.remove(&object.entity) {
            if let Some(name_tag_data) = create_nametag_data(
                &mut egui_context,
                &egui_managed_textures,
                &mut images,
                pending_name_tag_data,
            ) {
                name_tag_cache
                    .cache
                    .insert(object.name.name.clone(), name_tag_data);
                name_tag_cache.cache.get(&object.name.name).unwrap()
            } else {
                // Try again next frame
                continue;
            }
        } else {
            // Create egui text and wait until next frame to read the font texture to ensure
            // that the texture has been updated and contains the characters we want to use
            name_tag_cache.pending.insert(
                object.entity,
                create_pending_nametag(
                    &name_tag_settings,
                    &mut egui_context,
                    &object,
                    player.as_ref(),
                    name_tag_type,
                ),
            );
            continue;
        };

        // Spawn name tag entities
        let name_tag_entity = commands
            .spawn_bundle((
                NameTag { name_tag_type },
                Visibility {
                    is_visible: name_tag_settings.show_all[name_tag_type],
                },
                ComputedVisibility::default(),
                Transform::from_translation(Vec3::new(0.0, object.model_height.height, 0.0)),
                GlobalTransform::default(),
                NoFrustumCulling,
            ))
            .id();

        let target_mark = if let Some(npc_type_index) = object
            .npc
            .and_then(|npc| game_data.npcs.get_npc(npc.id))
            .and_then(|npc| npc.npc_type_index)
        {
            ui_resources
                .get_sprite_by_index(UiSpriteSheetType::TargetMark, npc_type_index.get() as usize)
                .zip(ui_resources.get_sprite_image_by_index(
                    UiSpriteSheetType::TargetMark,
                    npc_type_index.get() as usize,
                ))
        } else {
            None
        }
        .or_else(|| {
            ui_resources
                .get_sprite(0, "UI00_TARGETMARK")
                .zip(ui_resources.get_sprite_image(0, "UI00_TARGETMARK"))
        });

        let mut healthbar_fg_rect = None;
        let mut healthbar_bg_rect = None;
        let (health_foreground, health_background) = match name_tag_type {
            NameTagType::Character => (
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_RED_AVATAR")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_RED_AVATAR")),
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_BG_AVATAR")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_BG_AVATAR")),
            ),
            NameTagType::Monster => (
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_RED")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_RED")),
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_BACKGROUND")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_BACKGROUND")),
            ),
            NameTagType::Npc => (None, None),
        };

        let mut health_bar_size = Vec2::ZERO;
        let mut health_bar_foreground_uv_x_bounds = (0.0, 0.0);
        if let (
            Some((health_foreground_sprite, health_foreground_image)),
            Some((health_background_sprite, health_background_image)),
        ) = (health_foreground, health_background)
        {
            let bar_width = health_background_sprite.width * pixels_per_point;
            let bar_height = health_background_sprite.height * pixels_per_point;
            let bar_offset_y = if matches!(name_tag_type, NameTagType::Character) {
                // Character health bar is behind name
                name_tag_data.rects[0].screen_offset.y + name_tag_data.rects[0].screen_size.y / 2.0
                    - bar_height / 2.0
            } else {
                // Monster health bar under name
                name_tag_data.rects[0].screen_offset.y - bar_height
            };
            health_bar_size = Vec2::new(bar_width, bar_height);

            healthbar_bg_rect = Some(WorldUiRect {
                screen_offset: Vec2::new(-bar_width / 2.0, bar_offset_y),
                screen_size: Vec2::new(bar_width, bar_height),
                image: health_background_image.clone_weak(),
                uv_min: Vec2::new(
                    health_background_sprite.uv.min.x,
                    health_background_sprite.uv.min.y,
                ),
                uv_max: Vec2::new(
                    health_background_sprite.uv.max.x,
                    health_background_sprite.uv.max.y,
                ),
                color: Color::WHITE,
                order: ORDER_HEALTH_BACKGROUND,
            });

            health_bar_foreground_uv_x_bounds = (
                health_foreground_sprite.uv.min.x,
                health_foreground_sprite.uv.max.x,
            );
            healthbar_fg_rect = Some(WorldUiRect {
                screen_offset: Vec2::new(-bar_width / 2.0, bar_offset_y),
                screen_size: Vec2::new(bar_width, bar_height),
                image: health_foreground_image.clone_weak(),
                uv_min: Vec2::new(
                    health_foreground_sprite.uv.min.x,
                    health_foreground_sprite.uv.min.y,
                ),
                uv_max: Vec2::new(
                    health_foreground_sprite.uv.max.x,
                    health_foreground_sprite.uv.max.y,
                ),
                color: Color::WHITE,
                order: ORDER_HEALTH_FOREGROUND,
            });
        }

        let mut target_marks: ArrayVec<WorldUiRect, 2> = ArrayVec::default();
        if let Some((target_mark_sprite, target_mark_image)) = target_mark {
            let mark_width = target_mark_sprite.width * pixels_per_point;
            let mark_height = target_mark_sprite.height * pixels_per_point;
            let mark_offset_y =
                name_tag_data.rects[0].screen_offset.y + name_tag_data.rects[0].screen_size.y / 2.0;

            target_marks.push(WorldUiRect {
                screen_offset: Vec2::new(
                    name_tag_data.rects[0]
                        .screen_offset
                        .x
                        .min(-health_bar_size.x / 2.0)
                        - mark_width,
                    mark_offset_y - mark_height / 2.0,
                ),
                screen_size: Vec2::new(mark_width, mark_height),
                image: target_mark_image.clone_weak(),
                uv_min: Vec2::new(target_mark_sprite.uv.min.x, target_mark_sprite.uv.min.y),
                uv_max: Vec2::new(target_mark_sprite.uv.max.x, target_mark_sprite.uv.max.y),
                color: Color::WHITE,
                order: ORDER_TARGET_MARK,
            });

            target_marks.push(WorldUiRect {
                screen_offset: Vec2::new(
                    (name_tag_data.rects[0].screen_offset.x + name_tag_data.rects[0].screen_size.x)
                        .max(health_bar_size.x / 2.0),
                    mark_offset_y - mark_height / 2.0,
                ),
                screen_size: Vec2::new(mark_width, mark_height),
                image: target_mark_image.clone_weak(),
                uv_min: Vec2::new(target_mark_sprite.uv.max.x, target_mark_sprite.uv.min.y),
                uv_max: Vec2::new(target_mark_sprite.uv.min.x, target_mark_sprite.uv.max.y),
                color: Color::WHITE,
                order: ORDER_TARGET_MARK,
            });
        }

        commands.entity(name_tag_entity).add_children(|builder| {
            for rect in name_tag_data.rects.iter() {
                builder.spawn_bundle((
                    NameTagName,
                    rect.clone(),
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    ComputedVisibility::default(),
                    NoFrustumCulling,
                ));
            }

            for rect in target_marks.drain(..) {
                builder.spawn_bundle((
                    NameTagTargetMark,
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility { is_visible: false },
                    ComputedVisibility::default(),
                    NoFrustumCulling,
                ));
            }

            if let Some(rect) = healthbar_bg_rect.take() {
                builder.spawn_bundle((
                    NameTagHealthbarBackground,
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility { is_visible: false },
                    ComputedVisibility::default(),
                    NoFrustumCulling,
                ));
            }

            if let Some(rect) = healthbar_fg_rect.take() {
                builder.spawn_bundle((
                    NameTagHealthbarForeground {
                        full_width: health_bar_size.x,
                        uv_min_x: health_bar_foreground_uv_x_bounds.0,
                        uv_max_x: health_bar_foreground_uv_x_bounds.1,
                    },
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility { is_visible: false },
                    ComputedVisibility::default(),
                    NoFrustumCulling,
                ));
            }
        });

        commands
            .entity(object.entity)
            .insert(NameTagEntity(name_tag_entity))
            .add_child(name_tag_entity);
    }
}
