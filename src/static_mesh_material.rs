use crate::load_internal_asset;
use bevy::{
    asset::{AssetServer, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::Vec2,
    pbr::{AlphaMode, MaterialPipeline, MaterialPlugin, SpecializedMaterial},
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
            BufferInitDescriptor, BufferSize, BufferUsages, CompareFunction, FilterMode,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension, VertexFormat,
        },
        renderer::RenderDevice,
        texture::Image,
        RenderApp,
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

        app.add_plugin(MaterialPlugin::<StaticMeshMaterial>::default());

        let render_device = app.world.get_resource::<RenderDevice>().unwrap();
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(StaticMeshMaterialSamplers { linear_sampler });
        }
    }
}

pub struct StaticMeshMaterialSamplers {
    linear_sampler: Sampler,
}

// NOTE: These must match the bit flags in shaders/static_mesh_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct StaticMeshMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const ALPHA_MODE_BLEND           = (1 << 2);
        const HAS_ALPHA_VALUE            = (1 << 3);
        const NONE                       = 0;
    }
}

#[derive(Clone, AsStd140)]
pub struct StaticMeshMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6942088b-c082-457b-aacf-517228cc0c22"]
pub struct StaticMeshMaterial {
    pub base_texture: Handle<Image>,
    pub alpha_value: Option<f32>,
    pub alpha_mode: AlphaMode,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,

    // lightmap texture, uv offset, uv scale
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl Default for StaticMeshMaterial {
    fn default() -> Self {
        Self {
            base_texture: Default::default(),
            alpha_value: None,
            alpha_mode: AlphaMode::Opaque,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
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

    pub uniform_buffer: Buffer,
    pub base_texture: Handle<Image>,
    pub lightmap_texture: Option<Handle<Image>>,

    pub flags: StaticMeshMaterialFlags,
    pub alpha_value: Option<f32>,
    pub alpha_mode: AlphaMode,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
}

impl RenderAsset for StaticMeshMaterial {
    type ExtractedAsset = StaticMeshMaterial;
    type PreparedAsset = GpuStaticMeshMaterial;
    #[allow(clippy::type_complexity)]
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<StaticMeshMaterial>>,
        SRes<RenderAssets<Image>>,
        SRes<StaticMeshMaterialSamplers>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images, samplers): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let base_gpu_image = gpu_images.get(&material.base_texture);
        if base_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let base_texture_view = &base_gpu_image.unwrap().texture_view;
        let base_texture_sampler = &samplers.linear_sampler;

        let (lightmap_texture_view, _) = if let Some(result) = material_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.lightmap_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
        let lightmap_texture_sampler = &samplers.linear_sampler;

        let mut flags = StaticMeshMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        match material.alpha_mode {
            AlphaMode::Opaque => flags |= StaticMeshMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(c) => {
                alpha_cutoff = c;
                flags |= StaticMeshMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode::Blend => flags |= StaticMeshMaterialFlags::ALPHA_MODE_BLEND,
        };
        let alpha_value = if let Some(alpha_value) = material.alpha_value {
            flags |= StaticMeshMaterialFlags::HAS_ALPHA_VALUE;
            alpha_value
        } else {
            1.0
        };

        let value = StaticMeshMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff,
            alpha_value,
            lightmap_uv_offset: material.lightmap_uv_offset,
            lightmap_uv_scale: material.lightmap_uv_scale,
        };
        let value_std140 = value.as_std140();
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("static_mesh_material_uniform_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: value_std140.as_bytes(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(base_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(base_texture_sampler),
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
            uniform_buffer,
            base_texture: material.base_texture,
            lightmap_texture: material.lightmap_texture,
            flags,
            alpha_mode: material.alpha_mode,
            alpha_value: material.alpha_value,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StaticMeshMaterialKey {
    has_lightmap: bool,
    is_alpha_mask: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
}

impl SpecializedMaterial for StaticMeshMaterial {
    type Key = StaticMeshMaterialKey;

    fn key(render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        StaticMeshMaterialKey {
            has_lightmap: render_asset.lightmap_texture.is_some(),
            is_alpha_mask: matches!(render_asset.alpha_mode, AlphaMode::Mask(_)),
            two_sided: render_asset.two_sided,
            z_test_enabled: render_asset.z_test_enabled,
            z_write_enabled: render_asset.z_write_enabled,
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

        if key.two_sided {
            descriptor.primitive.cull_mode = None;
        }

        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = key.z_write_enabled;

        if !key.z_test_enabled {
            descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;
        }

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
                // Uniform data
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            StaticMeshMaterialUniformData::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                },
                // Base Texture
                BindGroupLayoutEntry {
                    binding: 1,
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
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
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
        if !render_asset.z_write_enabled {
            // When no depth write we need back to front rendering which only happens
            // in the the transparent pass, by returning AlphaMode::Blend here we tell
            // pbr::MeshPipeline to use the transparent pass
            AlphaMode::Blend
        } else {
            render_asset.alpha_mode
        }
    }
}
