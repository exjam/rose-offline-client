use bevy::{
    prelude::{AddAsset, App, Plugin},
    render::{mesh::MeshVertexAttribute, render_resource::VertexFormat},
};

mod damage_digit_material;
mod damage_digit_pipeline;
mod damage_digit_render_data;
mod effect_mesh_material;
mod object_material;
mod particle_material;
mod particle_pipeline;
mod particle_render_data;
mod rgb_texture_loader;
mod sky_material;
mod terrain_material;
mod texture_array;
mod trail_effect;
mod water_material;
mod world_ui;
mod zone_lighting;

pub const MESH_ATTRIBUTE_UV_1: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv2", 280035324, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_2: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv3", 2422131906, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_3: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv4", 519697814, VertexFormat::Float32x2);

pub use damage_digit_material::DamageDigitMaterial;
pub use damage_digit_render_data::DamageDigitRenderData;
pub use effect_mesh_material::{
    EffectMeshAnimationFlags, EffectMeshAnimationRenderState, EffectMeshMaterial,
};
pub use object_material::{
    ObjectMaterial, ObjectMaterialBlend, ObjectMaterialClipFace, ObjectMaterialGlow,
};
pub use particle_material::ParticleMaterial;
pub use particle_render_data::{ParticleRenderBillboardType, ParticleRenderData};
pub use rgb_texture_loader::RgbTextureLoader;
pub use sky_material::SkyMaterial;
pub use terrain_material::{
    TerrainMaterial, TERRAIN_MATERIAL_MAX_TEXTURES, TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
};
pub use texture_array::{GpuTextureArray, TextureArray, TextureArrayBuilder};
pub use trail_effect::TrailEffect;
pub use water_material::WaterMaterial;
pub use world_ui::WorldUiRect;
pub use zone_lighting::ZoneLighting;

use damage_digit_material::DamageDigitMaterialPlugin;
use damage_digit_pipeline::DamageDigitRenderPlugin;
use effect_mesh_material::EffectMeshMaterialPlugin;
use object_material::ObjectMaterialPlugin;
use particle_material::ParticleMaterialPlugin;
use particle_pipeline::ParticleRenderPlugin;
use sky_material::SkyMaterialPlugin;
use terrain_material::TerrainMaterialPlugin;
use texture_array::TextureArrayPlugin;
use trail_effect::TrailEffectRenderPlugin;
use water_material::WaterMaterialPlugin;
use world_ui::WorldUiRenderPlugin;
use zone_lighting::ZoneLightingPlugin;

#[derive(Default)]
pub struct RoseRenderPlugin;

impl Plugin for RoseRenderPlugin {
    fn build(&self, app: &mut App) {
        let prepass_enabled = false;

        app.add_plugin(TextureArrayPlugin)
            .add_plugin(ZoneLightingPlugin)
            .init_asset_loader::<RgbTextureLoader>();

        app.add_plugin(TerrainMaterialPlugin { prepass_enabled })
            .add_plugin(EffectMeshMaterialPlugin { prepass_enabled })
            .add_plugin(ObjectMaterialPlugin { prepass_enabled })
            .add_plugin(WaterMaterialPlugin { prepass_enabled })
            .add_plugin(ParticleMaterialPlugin)
            .add_plugin(ParticleRenderPlugin)
            .add_plugin(DamageDigitMaterialPlugin)
            .add_plugin(DamageDigitRenderPlugin)
            .add_plugin(SkyMaterialPlugin { prepass_enabled })
            .add_plugin(TrailEffectRenderPlugin)
            .add_plugin(WorldUiRenderPlugin);
    }
}
