use crate::{
    load_internal_asset,
    material::{AlphaMode, MaterialPipeline, SpecializedMaterial},
};
use bevy::{
    asset::{AssetServer, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::Vec2,
    prelude::{App, HandleUntyped, Mesh, Plugin},
    reflect::TypeUuid,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferSize, BufferUsages, RenderPipelineDescriptor,
            SamplerBindingType, ShaderStages, SpecializedMeshPipelineError, TextureSampleType,
            TextureViewDimension, VertexFormat,
        },
        renderer::RenderDevice,
        texture::Image,
    },
};

pub const STATIC_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 6942039651767701046);

pub const STATIC_MESH_ATTRIBUTE_UV1: MeshVertexAttribute = Mesh::ATTRIBUTE_UV_0;

pub const STATIC_MESH_ATTRIBUTE_UV2: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv2", 42069400, VertexFormat::Float32x2);

pub const STATIC_MESH_ATTRIBUTE_UV3: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv3", 42069401, VertexFormat::Float32x2);

pub const STATIC_MESH_ATTRIBUTE_UV4: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv4", 42069402, VertexFormat::Float32x2);

#[derive(Default)]
pub struct StaticMeshMaterialPlugin;

impl Plugin for StaticMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            STATIC_MESH_MATERIAL_SHADER_HANDLE,
            "shaders/static_mesh_material.wgsl",
            Shader::from_wgsl
        );
    }
}

#[derive(Clone, AsStd140)]
pub struct StaticMeshMaterialLightmapUniformData {
    pub uv_offset: Vec2,
    pub uv_scale: f32,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6942088b-c082-457b-aacf-517228cc0c22"]
pub struct StaticMeshMaterial {
    pub base_texture: Handle<Image>,
    pub alpha_mode: AlphaMode,

    // lightmap texture, uv offset, uv scale
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl Default for StaticMeshMaterial {
    fn default() -> Self {
        Self {
            base_texture: Default::default(),
            alpha_mode: AlphaMode::Opaque,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

/// The GPU representation of a [`StaticMeshMaterial`].
#[derive(Debug, Clone)]
pub struct GpuStaticMeshMaterial {
    pub bind_group: BindGroup,
    pub base_texture: Handle<Image>,
    pub alpha_mode: AlphaMode,
    pub has_lightmap: bool,
    pub lightmap_uniform_buffer: Buffer,
}

impl RenderAsset for StaticMeshMaterial {
    type ExtractedAsset = StaticMeshMaterial;
    type PreparedAsset = GpuStaticMeshMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<StaticMeshMaterial>>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (base_texture_view, base_texture_sampler) = if let Some(result) = material_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, Some(&material.base_texture))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (lightmap_texture_view, lightmap_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, material.lightmap_texture.as_ref())
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let value = StaticMeshMaterialLightmapUniformData {
            uv_offset: material.lightmap_uv_offset,
            uv_scale: material.lightmap_uv_scale,
        };
        let value_std140 = value.as_std140();
        let lightmap_uniform_buffer =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("static_mesh_material_uniform_buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: value_std140.as_bytes(),
            });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(base_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(base_texture_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: lightmap_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(lightmap_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(lightmap_texture_sampler),
                },
            ],
            label: Some("static_mesh_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuStaticMeshMaterial {
            bind_group,
            base_texture: material.base_texture,
            alpha_mode: material.alpha_mode,
            lightmap_uniform_buffer,
            has_lightmap: material.lightmap_texture.is_some(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StaticMeshMaterialKey {
    has_lightmap: bool,
}

impl SpecializedMaterial for StaticMeshMaterial {
    type Key = StaticMeshMaterialKey;

    fn key(render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        StaticMeshMaterialKey {
            has_lightmap: render_asset.has_lightmap,
        }
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            STATIC_MESH_ATTRIBUTE_UV1.at_shader_location(1),
        ];

        if key.has_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push(String::from("HAS_STATIC_MESH_LIGHTMAP"));
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(String::from("HAS_STATIC_MESH_LIGHTMAP"));

            vertex_attributes.push(STATIC_MESH_ATTRIBUTE_UV2.at_shader_location(2));
        }

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];
        Ok(())
    }

    fn vertex_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(STATIC_MESH_MATERIAL_SHADER_HANDLE.typed())
    }

    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(STATIC_MESH_MATERIAL_SHADER_HANDLE.typed())
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
                // Base Texture
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
                // Base Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Lightmap uniform data
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            StaticMeshMaterialLightmapUniformData::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                },
                // Lightmap Texture
                BindGroupLayoutEntry {
                    binding: 3,
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
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("static_mesh_material_layout"),
        })
    }

    #[inline]
    fn alpha_mode(render_asset: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        render_asset.alpha_mode
    }
}
