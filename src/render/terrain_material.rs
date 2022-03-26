use bevy::{
    asset::{AssetServer, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{MaterialPipeline, MaterialPlugin, SpecializedMaterial},
    prelude::{App, Assets, HandleUntyped, Mesh, Plugin},
    reflect::TypeUuid,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, FilterMode,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension, VertexFormat,
        },
        renderer::RenderDevice,
        texture::Image,
        RenderApp,
    },
};

use crate::render::{TextureArray, MESH_ATTRIBUTE_UV_1};

pub const TERRAIN_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x3d7939250aff89cb);

pub const TERRAIN_MESH_ATTRIBUTE_TILE_INFO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileInfo", 3855645392, VertexFormat::Sint32x3);

#[derive(Default)]
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            TERRAIN_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/terrain_material.wgsl")),
        );

        app.add_plugin(MaterialPlugin::<TerrainMaterial>::default());

        let render_device = app.world.resource::<RenderDevice>();
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(TerrainMaterialSamplers { linear_sampler });
        }
    }
}

pub struct TerrainMaterialSamplers {
    linear_sampler: Sampler,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "403e3628-46d2-4d2a-b74c-ce84be2b1ba2"]
pub struct TerrainMaterial {
    pub lightmap_texture: Handle<Image>,
    pub tile_array_texture: Handle<TextureArray>,
}

/// The GPU representation of a [`TerrainMaterial`].
#[derive(Debug, Clone)]
pub struct GpuTerrainMaterial {
    pub bind_group: BindGroup,
    pub lightmap_texture: Handle<Image>,
    pub tile_array_texture: Handle<TextureArray>,
}

impl RenderAsset for TerrainMaterial {
    type ExtractedAsset = TerrainMaterial;
    type PreparedAsset = GpuTerrainMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<TerrainMaterial>>,
        SRes<RenderAssets<Image>>,
        SRes<RenderAssets<TextureArray>>,
        SRes<TerrainMaterialSamplers>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images, gpu_texture_arrays, samplers): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let lightmap_gpu_image = gpu_images.get(&material.lightmap_texture);
        if lightmap_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let lightmap_texture_view = &lightmap_gpu_image.unwrap().texture_view;
        let lightmap_texture_sampler = &samplers.linear_sampler;

        let tile_array_gpu_image = gpu_texture_arrays.get(&material.tile_array_texture);
        if tile_array_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let tile_array_texture_view = &tile_array_gpu_image.unwrap().texture_view;
        let tile_array_texture_sampler = &samplers.linear_sampler;

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(lightmap_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(lightmap_texture_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(tile_array_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(tile_array_texture_sampler),
                },
            ],
            label: Some("pbr_standard_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuTerrainMaterial {
            bind_group,
            lightmap_texture: material.lightmap_texture.clone(),
            tile_array_texture: material.tile_array_texture.clone(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TerrainMaterialKey;

impl SpecializedMaterial for TerrainMaterial {
    type Key = TerrainMaterialKey;

    fn key(_render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        TerrainMaterialKey {}
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            MESH_ATTRIBUTE_UV_1.at_shader_location(2),
            TERRAIN_MESH_ATTRIBUTE_TILE_INFO.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }

    fn vertex_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(TERRAIN_MATERIAL_SHADER_HANDLE.typed())
    }

    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(TERRAIN_MATERIAL_SHADER_HANDLE.typed())
    }

    #[inline]
    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(
        render_device: &RenderDevice,
    ) -> bevy::render::render_resource::BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Lightmap Texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Lightmap Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Tile Array Texture
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Tile Array Texture Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("terrain_material_layout"),
        })
    }
}
