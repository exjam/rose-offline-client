use bevy::{
    asset::{AssetServer, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::Vec2,
    pbr::{AlphaMode, MaterialPipeline, MaterialPlugin, SpecializedMaterial},
    prelude::{App, Assets, HandleUntyped, Mesh, Plugin},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferSize, BufferUsages, CompareFunction, FilterMode,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::Image,
        RenderApp,
    },
};

use crate::render::MESH_ATTRIBUTE_UV_1;

pub const STATIC_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xb7ebbc00ea16d3c7);

#[derive(Default)]
pub struct StaticMeshMaterialPlugin;

impl Plugin for StaticMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            STATIC_MESH_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/static_mesh_material.wgsl")),
        );

        app.add_plugin(MaterialPlugin::<StaticMeshMaterial>::default());

        let render_device = app.world.resource::<RenderDevice>();
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
#[uuid = "62a496fa-33e8-41a8-9a44-237d70214227"]
pub struct StaticMeshMaterial {
    pub base_texture: Option<Handle<Image>>,
    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
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
            base_texture: None,
            alpha_value: None,
            alpha_enabled: false,
            alpha_test: None,
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
    pub base_texture: Option<Handle<Image>>,
    pub lightmap_texture: Option<Handle<Image>>,

    pub flags: StaticMeshMaterialFlags,
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
        let (base_texture_view, _) = if let Some(result) = material_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.base_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
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
        let mut alpha_mode = AlphaMode::Opaque;

        if material.alpha_enabled {
            flags |= StaticMeshMaterialFlags::ALPHA_MODE_BLEND;
            alpha_mode = AlphaMode::Blend;
        }

        if let Some(alpha_ref) = material.alpha_test {
            flags |= StaticMeshMaterialFlags::ALPHA_MODE_MASK;
            alpha_cutoff = alpha_ref;
            alpha_mode = AlphaMode::Mask(alpha_cutoff);
        }

        if !material.alpha_enabled && material.alpha_test.is_none() {
            flags |= StaticMeshMaterialFlags::ALPHA_MODE_OPAQUE;
        }

        let alpha_value = if let Some(alpha_value) = material.alpha_value {
            flags |= StaticMeshMaterialFlags::HAS_ALPHA_VALUE;
            alpha_mode = AlphaMode::Blend;
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
            alpha_mode,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StaticMeshMaterialKey {
    has_lightmap: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
}

impl SpecializedMaterial for StaticMeshMaterial {
    type Key = StaticMeshMaterialKey;

    fn key(render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        StaticMeshMaterialKey {
            has_lightmap: render_asset.lightmap_texture.is_some(),
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
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
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

            vertex_attributes.push(MESH_ATTRIBUTE_UV_1.at_shader_location(2));
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
