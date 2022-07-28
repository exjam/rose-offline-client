use std::collections::HashMap;

use bevy::{
    asset::HandleId,
    prelude::{AssetServer, Commands, Handle, Image, Res, ResMut},
};
use bevy_egui::{egui::TextureId, EguiContext};
use enum_map::{enum_map, Enum, EnumMap};

use rose_file_readers::{IdFile, TsiFile, TsiSprite, TsiTexture, VfsIndex};

use crate::{ui::Dialog, VfsResource};

#[derive(Enum)]
pub enum UiSpriteSheetType {
    Ui,
    ExUi,
}

pub struct UiSpriteSheet {
    pub textures: Vec<TsiTexture>,
    pub sprites: Vec<TsiSprite>,
    pub loaded_textures: Vec<(Handle<Image>, TextureId)>,
    pub sprites_by_name: IdFile,
}

pub struct UiResources {
    pub sprite_sheets: EnumMap<UiSpriteSheetType, UiSpriteSheet>,
    pub sprite_sheets_load_group: Vec<HandleId>,
    pub dialog_files: HashMap<String, Handle<Dialog>>,
    pub dialog_login: Handle<Dialog>,
    pub dialog_character_info: Handle<Dialog>,
    pub dialog_game_menu: Handle<Dialog>,
    pub dialog_player_info: Handle<Dialog>,
}

fn load_ui_spritesheet(
    vfs: &VfsIndex,
    asset_server: &AssetServer,
    egui_context: &mut EguiContext,
    load_group: &mut Vec<HandleId>,
    tsi_path: &str,
    id_path: &str,
) -> Result<UiSpriteSheet, anyhow::Error> {
    let tsi_file = vfs.read_file::<TsiFile, _>(tsi_path)?;
    let id_file = vfs.read_file::<IdFile, _>(id_path)?;

    let mut loaded_textures = Vec::new();
    for tsi_texture in tsi_file.textures.iter() {
        let image_handle =
            asset_server.load(&format!("3DDATA/CONTROL/RES/{}", tsi_texture.filename));
        let texture_id = egui_context.add_image(image_handle.clone_weak());
        load_group.push(image_handle.id);
        loaded_textures.push((image_handle, texture_id));
    }

    Ok(UiSpriteSheet {
        textures: tsi_file.textures,
        sprites: tsi_file.sprites,
        loaded_textures,
        sprites_by_name: id_file,
    })
}

pub fn load_ui_resources(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    mut egui_context: ResMut<EguiContext>,
) {
    let vfs = &vfs_resource.vfs;
    let mut load_group = Vec::new();

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
        sprite_sheets: enum_map! {
            UiSpriteSheetType::Ui => load_ui_spritesheet(vfs, &asset_server, &mut egui_context, &mut load_group, "3DDATA/CONTROL/RES/UI.TSI", "3DDATA/CONTROL/XML/UI_STRID.ID").expect("Failed to load UI sprite sheet"),
            UiSpriteSheetType::ExUi => load_ui_spritesheet(vfs, &asset_server, &mut egui_context, &mut load_group,  "3DDATA/CONTROL/RES/EXUI.TSI", "3DDATA/CONTROL/XML/EXUI_STRID.ID").expect("Failed to load EXUI sprite sheet"),
        },
        sprite_sheets_load_group: load_group,
        dialog_character_info: dialog_files["DLGAVATA.XML"].clone(),
        dialog_game_menu: dialog_files["DLGMENU.XML"].clone(),
        dialog_login: dialog_files["DLGLOGIN.XML"].clone(),
        dialog_player_info: dialog_files["DLGINFO.XML"].clone(),
        dialog_files,
    });
}
