use crate::{
    load_internal_asset,
    material::{MaterialPipeline, SpecializedMaterial},
    mesh_pipeline::{
        MESH_ATTRIBUTE_POSITION, MESH_ATTRIBUTE_TERRAIN_TILE_INFO, MESH_ATTRIBUTE_UV1,
        MESH_ATTRIBUTE_UV2,
    },
};
use bevy::{
    asset::{AssetServer, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::{App, HandleUntyped, Plugin},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, RenderPipelineDescriptor,
            SamplerBindingType, ShaderStages, SpecializedMeshPipelineError, TextureSampleType,
            TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::Image,
    },
};

pub const TERRAIN_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4206939651767701046);

#[derive(Default)]
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TERRAIN_MATERIAL_SHADER_HANDLE,
            "shaders/terrain_material.wgsl",
            Shader::from_wgsl
        );
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7494888b-c082-457b-aacf-517228cc0c22"]
pub struct TerrainMaterial {
    pub lightmap_texture: Handle<Image>,
    pub tile_array_texture: Handle<Image>,
}

/// The GPU representation of a [`TerrainMaterial`].
#[derive(Debug, Clone)]
pub struct GpuTerrainMaterial {
    pub bind_group: BindGroup,
    pub lightmap_texture: Handle<Image>,
    pub tile_array_texture: Handle<Image>,
}

impl RenderAsset for TerrainMaterial {
    type ExtractedAsset = TerrainMaterial;
    type PreparedAsset = GpuTerrainMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<TerrainMaterial>>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (lightmap_texture_view, lightmap_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, Some(&material.lightmap_texture))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (tile_array_texture_view, tile_array_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, Some(&material.tile_array_texture))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

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
            lightmap_texture: material.lightmap_texture,
            tile_array_texture: material.tile_array_texture,
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
            MESH_ATTRIBUTE_POSITION.at_shader_location(0),
            MESH_ATTRIBUTE_UV1.at_shader_location(1),
            MESH_ATTRIBUTE_UV2.at_shader_location(2),
            MESH_ATTRIBUTE_TERRAIN_TILE_INFO.at_shader_location(3),
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
