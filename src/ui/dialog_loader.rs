use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    prelude::{AssetEvent, Assets, EventReader, Handle, Local, Res, ResMut},
};

use quick_xml::de::from_slice;

use crate::{
    resources::UiResources,
    ui::widgets::{Dialog, LoadWidget},
};

#[derive(Default)]
pub struct DialogLoader;

impl AssetLoader for DialogLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let dialog: Dialog = from_slice(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(dialog));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["xml"]
    }
}

pub struct DialogInstance {
    pub filename: String,
    pub instance: Option<Dialog>,
}

impl DialogInstance {
    pub fn new(filename: impl Into<String>) -> DialogInstance {
        DialogInstance {
            filename: filename.into(),
            instance: None,
        }
    }

    pub fn get_mut(
        &mut self,
        dialog_assets: &Assets<Dialog>,
        ui_resources: &UiResources,
    ) -> Option<&mut Dialog> {
        if self.instance.is_none() {
            if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_files[&self.filename]) {
                if dialog.loaded {
                    self.instance = Some(dialog.clone());
                }
            }
        }

        self.instance.as_mut()
    }
}

#[derive(Default)]
pub struct DialogsLoadState {
    pending_dialogs: Vec<Handle<Dialog>>,
}

pub fn load_dialog_sprites_system(
    mut ev_asset: EventReader<AssetEvent<Dialog>>,
    mut assets: ResMut<Assets<Dialog>>,
    mut load_state: Local<DialogsLoadState>,
    ui_resources: Res<UiResources>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                load_state.pending_dialogs.push(handle.clone_weak());
            }
            _ => {}
        }
    }

    if ui_resources.loaded_all_textures {
        for handle in load_state.pending_dialogs.drain(..) {
            if let Some(dialog) = assets.get_mut(&handle) {
                dialog.widgets.load_widget(&ui_resources);
                dialog.loaded = true;
            }
        }
    }
}
