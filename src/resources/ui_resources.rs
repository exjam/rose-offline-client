use std::collections::HashMap;

use bevy::{
    asset::LoadState,
    prelude::{AssetServer, Assets, Commands, Handle, Image, Res, ResMut, Vec2},
};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, Enum, EnumMap};

use rose_file_readers::{IdFile, TsiFile, TsiSprite, VirtualFilesystem};

use crate::{ui::widgets::Dialog, VfsResource};

#[derive(Clone)]
pub struct UiSprite {
    pub texture_id: egui::TextureId,
    pub uv: egui::Rect,
    pub width: f32,
    pub height: f32,
}

impl UiSprite {
    pub fn draw(&self, ui: &mut egui::Ui, pos: egui::Pos2) {
        let rect = egui::Rect::from_min_size(pos, egui::vec2(self.width, self.height));
        let mut mesh = egui::epaint::Mesh::with_texture(self.texture_id);
        mesh.add_rect_with_uv(rect, self.uv, egui::Color32::WHITE);
        ui.painter().add(egui::epaint::Shape::mesh(mesh));
    }

    pub fn draw_stretched(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let mut mesh = egui::epaint::Mesh::with_texture(self.texture_id);
        mesh.add_rect_with_uv(rect, self.uv, egui::Color32::WHITE);
        ui.painter().add(egui::epaint::Shape::mesh(mesh));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Enum)]
pub enum UiSpriteSheetType {
    Ui,
    ExUi,
    Item,
    Skill,
    StateIcon,
    ItemSocket,
    MinimapArrow,
}

pub struct UiTexture {
    pub handle: Handle<Image>,
    pub texture_id: egui::TextureId,
    pub size: Option<Vec2>,
}

pub struct UiSpriteSheet {
    pub sprites: Vec<TsiSprite>,
    pub loaded_textures: Vec<UiTexture>,
    pub sprites_by_name: Option<IdFile>,
}

pub struct UiResources {
    pub loaded_all_textures: bool,
    pub sprite_sheets: EnumMap<UiSpriteSheetType, Option<UiSpriteSheet>>,

    pub dialog_files: HashMap<String, Handle<Dialog>>,
    pub dialog_login: Handle<Dialog>,
    pub dialog_character_info: Handle<Dialog>,
    pub dialog_chatbox: Handle<Dialog>,
    pub dialog_create_avatar: Handle<Dialog>,
    pub dialog_game_menu: Handle<Dialog>,
    pub dialog_message_box: Handle<Dialog>,
    pub dialog_minimap: Handle<Dialog>,
    pub dialog_party: Handle<Dialog>,
    pub dialog_party_option: Handle<Dialog>,
    pub dialog_personal_store: Handle<Dialog>,
    pub dialog_player_info: Handle<Dialog>,
    pub dialog_select_server: Handle<Dialog>,
    pub dialog_skill_list: Handle<Dialog>,
}

impl UiResources {
    pub fn get_sprite(&self, module_id: i32, sprite_name: &str) -> Option<UiSprite> {
        let sprite_sheet_type = match module_id {
            0 => UiSpriteSheetType::Ui,
            1 => UiSpriteSheetType::Item,
            3 => UiSpriteSheetType::ExUi,
            4 => UiSpriteSheetType::Skill,
            5 => UiSpriteSheetType::StateIcon,
            6 => UiSpriteSheetType::ItemSocket,
            _ => return None,
        };
        let sprite_sheet = self.sprite_sheets[sprite_sheet_type].as_ref()?;
        let sprite_index = sprite_sheet
            .sprites_by_name
            .as_ref()
            .unwrap()
            .get(sprite_name)?;

        self.get_sprite_by_index(sprite_sheet_type, *sprite_index as usize)
    }

    pub fn get_sprite_by_index(
        &self,
        sprite_sheet_type: UiSpriteSheetType,
        sprite_index: usize,
    ) -> Option<UiSprite> {
        let sprite_sheet = self.sprite_sheets[sprite_sheet_type].as_ref()?;
        let sprite = sprite_sheet.sprites.get(sprite_index)?;
        let texture = sprite_sheet
            .loaded_textures
            .get(sprite.texture_id as usize)?;
        let texture_size = texture.size?;

        Some(UiSprite {
            texture_id: texture.texture_id,
            uv: egui::Rect::from_min_max(
                egui::pos2(
                    (sprite.left as f32 + 0.5) / texture_size.x,
                    (sprite.top as f32 + 0.5) / texture_size.y,
                ),
                egui::pos2(
                    (sprite.right as f32 + 0.5) / texture_size.x,
                    (sprite.bottom as f32 + 0.5) / texture_size.y,
                ),
            ),
            width: ((sprite.right + 1) - sprite.left) as f32,
            height: ((sprite.bottom + 1) - sprite.top) as f32,
        })
    }

    pub fn get_minimap_player_sprite(&self) -> Option<UiSprite> {
        let texture = &self.sprite_sheets[UiSpriteSheetType::MinimapArrow]
            .as_ref()?
            .loaded_textures[0];
        let texture_size = texture.size?;

        Some(UiSprite {
            texture_id: self.sprite_sheets[UiSpriteSheetType::MinimapArrow]
                .as_ref()?
                .loaded_textures[0]
                .texture_id,
            uv: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            width: texture_size.x,
            height: texture_size.y,
        })
    }
}

fn load_ui_spritesheet(
    vfs: &VirtualFilesystem,
    asset_server: &AssetServer,
    egui_context: &mut EguiContext,
    tsi_path: &str,
    id_path: &str,
) -> Result<UiSpriteSheet, anyhow::Error> {
    let tsi_file = vfs.read_file::<TsiFile, _>(tsi_path)?;
    let id_file = if id_path.is_empty() {
        None
    } else {
        Some(vfs.read_file::<IdFile, _>(id_path)?)
    };

    let mut loaded_textures = Vec::new();
    for tsi_texture in tsi_file.textures.iter() {
        let handle = asset_server.load(&format!("3DDATA/CONTROL/RES/{}", tsi_texture.filename));
        let texture_id = egui_context.add_image(handle.clone_weak());
        loaded_textures.push(UiTexture {
            handle,
            texture_id,
            size: None,
        });
    }

    Ok(UiSpriteSheet {
        sprites: tsi_file.sprites,
        loaded_textures,
        sprites_by_name: id_file,
    })
}

pub fn update_ui_resources(
    mut ui_resources: ResMut<UiResources>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    if ui_resources.loaded_all_textures {
        return;
    }

    let mut loaded_all = true;

    for spritesheet in ui_resources
        .sprite_sheets
        .iter_mut()
        .filter_map(|(_, spritesheet)| spritesheet.as_mut())
    {
        for texture in spritesheet.loaded_textures.iter_mut() {
            if texture.size.is_some() {
                continue;
            }

            if let Some(image) = images.get(&texture.handle) {
                texture.size = Some(image.size());
            } else if matches!(
                asset_server.get_load_state(&texture.handle),
                LoadState::Failed
            ) {
                texture.size = Some(Vec2::ZERO);
            } else {
                loaded_all = false;
            }
        }
    }

    ui_resources.loaded_all_textures = loaded_all;
}

pub fn load_ui_resources(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    mut egui_context: ResMut<EguiContext>,
) {
    let vfs = &vfs_resource.vfs;

    let dialog_filenames = [
        "DELIVERYSTORE.XML",
        "DLGADDFRIEND.XML",
        "DLGAVATA.XML",
        "DLGAVATARSTORE.XML",
        "DLGBANK.XML",
        "DLGCHAT.XML",
        "DLGCHATFILTER.XML",
        "DLGCHATROOM.XML",
        "DLGCLAN.XML",
        "DLGCLANREGNOTICE.XML",
        "DLGCOMM.XML",
        "DLGCREATEAVATAR.XML",
        "DLGDEAL.XML",
        "DLGDIALOG.XML",
        "DLGDIALOGEVENT.XML",
        "DLGEXCHANGE.XML",
        "DLGGOODS.XML",
        "DLGHELP.XML",
        "DLGINFO.XML",
        "DLGINPUTNAME.XML",
        "DLGITEM.XML",
        "DLGLOGIN.XML",
        "DLGMAKE.XML",
        "DLGMEMO.XML",
        "DLGMEMOVIEW.XML",
        "DLGMENU.XML",
        "DLGMINIMAP.XML",
        "DLGNINPUT.XML",
        "DLGNOTIFY.XML",
        "DLGOPTION.XML",
        "DLGORGANIZECLAN.XML",
        "DLGPARTY.XML",
        "DLGPARTYOPTION.XML",
        "DLGPRIVATECHAT.XML",
        "DLGPRIVATESTORE.XML",
        "DLGQUEST.XML",
        "DLGQUICKBAR.XML",
        "DLGRESTART.XML",
        "DLGSELAVATAR.XML",
        "DLGSELECTEVENT.XML",
        "DLGSELONLYSVR.XML",
        "DLGSELSVR.XML",
        "DLGSEPARATE.XML",
        "DLGSKILL.XML",
        "DLGSKILLTREE.XML",
        "DLGSTORE.XML",
        "DLGSYSTEM.XML",
        "DLGSYSTEMMSG.XML",
        "DLGUPGRADE.XML",
        "MSGBOX.XML",
        "SKILLTREE_DEALER.XML",
        "SKILLTREE_HOWKER.XML",
        "SKILLTREE_MUSE.XML",
        "SKILLTREE_SOLDIER.XML",
    ];

    let mut dialog_files = HashMap::new();
    for filename in dialog_filenames {
        dialog_files.insert(
            filename.to_string(),
            asset_server.load(&format!("3DDATA/CONTROL/XML/{}", filename)),
        );
    }

    commands.insert_resource(UiResources {
        loaded_all_textures: false,
        sprite_sheets: enum_map! {
            UiSpriteSheetType::Ui => load_ui_spritesheet(vfs, &asset_server, &mut egui_context, "3DDATA/CONTROL/RES/UI.TSI", "3DDATA/CONTROL/XML/UI_STRID.ID").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::ExUi => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3DDATA/CONTROL/RES/EXUI.TSI", "3DDATA/CONTROL/XML/EXUI_STRID.ID").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::StateIcon => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3DDATA/CONTROL/RES/STATEICON.TSI", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::Skill => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3DDATA/CONTROL/RES/SKILLICON.TSI", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::Item => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3DDATA/CONTROL/RES/ITEM1.TSI", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::ItemSocket => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3DDATA/CONTROL/RES/SOKETJAM.TSI", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::MinimapArrow => {
                let handle = asset_server.load("3DDATA/CONTROL/RES/MINIMAP_ARROW.TGA");
                let texture_id = egui_context.add_image(handle.clone_weak());

                Some(UiSpriteSheet {
                    sprites: vec![
                        TsiSprite { texture_id: 0, left: 0, top: 0, right: 0, bottom: 0, name: String::default() },
                    ],
                    loaded_textures: vec![
                        UiTexture { handle, texture_id, size: None },
                    ],
                    sprites_by_name: None,
                })
            }
        },
        dialog_character_info: dialog_files["DLGAVATA.XML"].clone(),
        dialog_chatbox: dialog_files["DLGCHAT.XML"].clone(),
        dialog_create_avatar: dialog_files[
            "DLGCREATEAVATAR.XML"].clone(),
        dialog_game_menu: dialog_files["DLGMENU.XML"].clone(),
        dialog_login: dialog_files["DLGLOGIN.XML"].clone(),
        dialog_message_box: dialog_files["MSGBOX.XML"].clone(),
        dialog_minimap: dialog_files["DLGMINIMAP.XML"].clone(),
        dialog_party: dialog_files["DLGPARTY.XML"].clone(),
        dialog_party_option: dialog_files["DLGPARTYOPTION.XML"].clone(),
        dialog_personal_store: dialog_files["DLGAVATARSTORE.XML"].clone(),
        dialog_player_info: dialog_files["DLGINFO.XML"].clone(),
        dialog_select_server: dialog_files["DLGSELSVR.XML"].clone(),
        dialog_skill_list: dialog_files["DLGSKILL.XML"].clone(),
        dialog_files,
    });
}
