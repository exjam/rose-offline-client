use bevy::{
    app::{App, Plugin},
    asset::{AddAsset, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
        renderer::RenderDevice,
        texture::Image,
    },
};

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "0078f73d-8715-427e-aa65-dc8e1f485d3d"]
pub struct ParticleMaterial {
    pub texture: Handle<Image>,
}

pub struct ParticleMaterialPlugin;

impl Plugin for ParticleMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenderAssetPlugin::<ParticleMaterial>::default())
            .add_asset::<ParticleMaterial>();
    }
}

#[derive(Debug, Clone)]
pub struct GpuParticleMaterial {
    pub texture: Handle<Image>,
}

impl RenderAsset for ParticleMaterial {
    type ExtractedAsset = ParticleMaterial;
    type PreparedAsset = GpuParticleMaterial;
    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        _render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        Ok(GpuParticleMaterial {
            texture: material.texture,
        })
    }
}
