use bevy::{
    app::{App, Plugin},
    asset::{AddAsset, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    reflect::{TypePath, TypeUuid},
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
        renderer::RenderDevice,
        texture::Image,
    },
};

#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "83077909-bf71-4f14-9a86-16f65d611ce9"]
pub struct DamageDigitMaterial {
    pub texture: Handle<Image>,
}

pub struct DamageDigitMaterialPlugin;

impl Plugin for DamageDigitMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<DamageDigitMaterial>::default())
            .add_asset::<DamageDigitMaterial>();
    }
}

#[derive(Debug, Clone)]
pub struct GpuDamageDigitMaterial {
    pub texture: Handle<Image>,
}

impl RenderAsset for DamageDigitMaterial {
    type ExtractedAsset = DamageDigitMaterial;
    type PreparedAsset = GpuDamageDigitMaterial;
    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        _render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        Ok(GpuDamageDigitMaterial {
            texture: material.texture,
        })
    }
}
