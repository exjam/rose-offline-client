use bevy::{
    prelude::{App, Plugin},
    render::{mesh::MeshVertexAttribute, render_resource::VertexFormat},
};

mod damage_digit_material;
mod damage_digit_pipeline;
mod damage_digit_render_data;
mod effect_mesh_material;
mod particle_material;
mod particle_pipeline;
mod particle_render_data;
mod sky_material;
mod static_mesh_material;
mod terrain_material;
mod texture_array;
mod water_mesh_material;

pub const MESH_ATTRIBUTE_UV_1: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv2", 280035324, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_2: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv3", 2422131906, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_3: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv4", 519697814, VertexFormat::Float32x2);

pub use damage_digit_material::DamageDigitMaterial;
pub use damage_digit_render_data::DamageDigitRenderData;
pub use effect_mesh_material::EffectMeshMaterial;
pub use particle_material::ParticleMaterial;
pub use particle_render_data::{ParticleRenderBillboardType, ParticleRenderData};
pub use sky_material::SkyMaterial;
pub use static_mesh_material::StaticMeshMaterial;
pub use terrain_material::{TerrainMaterial, TERRAIN_MESH_ATTRIBUTE_TILE_INFO};
pub use texture_array::{GpuTextureArray, TextureArray, TextureArrayBuilder};
pub use water_mesh_material::WaterMeshMaterial;

use damage_digit_material::DamageDigitMaterialPlugin;
use damage_digit_pipeline::DamageDigitRenderPlugin;
use effect_mesh_material::EffectMeshMaterialPlugin;
use particle_material::ParticleMaterialPlugin;
use particle_pipeline::ParticleRenderPlugin;
use sky_material::SkyMaterialPlugin;
use static_mesh_material::StaticMeshMaterialPlugin;
use terrain_material::TerrainMaterialPlugin;
use texture_array::TextureArrayPlugin;
use water_mesh_material::WaterMeshMaterialPlugin;

#[derive(Default)]
pub struct RoseRenderPlugin;

impl Plugin for RoseRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(TextureArrayPlugin)
            .add_plugin(TerrainMaterialPlugin)
            .add_plugin(EffectMeshMaterialPlugin)
            .add_plugin(StaticMeshMaterialPlugin)
            .add_plugin(WaterMeshMaterialPlugin)
            .add_plugin(ParticleMaterialPlugin)
            .add_plugin(ParticleRenderPlugin)
            .add_plugin(DamageDigitMaterialPlugin)
            .add_plugin(DamageDigitRenderPlugin)
            .add_plugin(SkyMaterialPlugin);
    }
}
