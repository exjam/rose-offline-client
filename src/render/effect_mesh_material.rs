use bevy::{
    asset::{AssetServer, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
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
            BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
            BlendOperation, BlendState, Buffer, BufferBindingType, BufferInitDescriptor,
            BufferSize, BufferUsages, CompareFunction, FilterMode, RenderPipelineDescriptor,
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::Image,
        RenderApp,
    },
};

pub const EFFECT_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x90d5233c3001d33e);

#[derive(Default)]
pub struct EffectMeshMaterialPlugin;

impl Plugin for EffectMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            EFFECT_MESH_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/effect_mesh_material.wgsl")),
        );

        app.add_plugin(MaterialPlugin::<EffectMeshMaterial>::default());

        let render_device = app.world.resource::<RenderDevice>();
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(EffectMeshMaterialSamplers { linear_sampler });
        }
    }
}

pub struct EffectMeshMaterialSamplers {
    linear_sampler: Sampler,
}

// NOTE: These must match the bit flags in shaders/effect_mesh_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct EffectMeshMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const NONE                       = 0;
    }
}

#[derive(Clone, AsStd140)]
pub struct EffectMeshMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "9ac3266d-1aa6-4f67-ade4-e3765fd0b1a1"]
pub struct EffectMeshMaterial {
    pub base_texture: Option<Handle<Image>>,

    pub alpha_enabled: bool,
    pub alpha_test: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

/// The GPU representation of a [`EffectMeshMaterial`].
#[derive(Debug, Clone)]
pub struct GpuEffectMeshMaterial {
    pub bind_group: BindGroup,
    pub base_texture: Option<Handle<Image>>,
    pub uniform_buffer: Buffer,

    pub alpha_enabled: bool,
    pub alpha_test: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

impl RenderAsset for EffectMeshMaterial {
    type ExtractedAsset = EffectMeshMaterial;
    type PreparedAsset = GpuEffectMeshMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<EffectMeshMaterial>>,
        SRes<RenderAssets<Image>>,
        SRes<EffectMeshMaterialSamplers>,
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

        let mut flags = EffectMeshMaterialFlags::NONE;
        if material.alpha_test {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_MASK;
        } else if !material.alpha_enabled {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_OPAQUE;
        }

        let value = EffectMeshMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff: 0.5,
        };
        let value_std140 = value.as_std140();
        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("effect_mesh_material_uniform_buffer"),
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
            ],
            label: Some("effect_mesh_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuEffectMeshMaterial {
            bind_group,
            uniform_buffer,
            base_texture: material.base_texture,
            alpha_enabled: material.alpha_enabled,
            alpha_test: material.alpha_test,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            blend_op: material.blend_op,
            src_blend_factor: material.src_blend_factor,
            dst_blend_factor: material.dst_blend_factor,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EffectMeshMaterialKey {
    alpha_enabled: bool,
    alpha_test: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
    blend_op: BlendOperation,
    src_blend_factor: BlendFactor,
    dst_blend_factor: BlendFactor,
}

impl SpecializedMaterial for EffectMeshMaterial {
    type Key = EffectMeshMaterialKey;

    fn key(render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        EffectMeshMaterialKey {
            alpha_enabled: render_asset.alpha_enabled,
            alpha_test: render_asset.alpha_test,
            two_sided: render_asset.two_sided,
            z_test_enabled: render_asset.z_test_enabled,
            z_write_enabled: render_asset.z_write_enabled,
            blend_op: render_asset.blend_op,
            src_blend_factor: render_asset.src_blend_factor,
            dst_blend_factor: render_asset.dst_blend_factor,
        }
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_attributes = [
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ];

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];
        descriptor.fragment.as_mut().unwrap().targets[0].blend = Some(BlendState {
            color: BlendComponent {
                src_factor: key.src_blend_factor,
                dst_factor: key.dst_blend_factor,
                operation: key.blend_op,
            },
            alpha: BlendComponent {
                src_factor: key.src_blend_factor,
                dst_factor: key.dst_blend_factor,
                operation: key.blend_op,
            },
        });

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
        Some(EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed())
    }

    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed())
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
                            EffectMeshMaterialUniformData::std140_size_static() as u64,
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
            ],
            label: Some("effect_mesh_material_layout"),
        })
    }

    #[inline]
    fn alpha_mode(render_asset: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        if render_asset.alpha_enabled || !render_asset.z_write_enabled {
            // When no depth write we need back to front rendering which only happens
            // in the the transparent pass, by returning AlphaMode::Blend here we tell
            // pbr::MeshPipeline to use the transparent pass
            AlphaMode::Blend
        } else if render_asset.alpha_test {
            AlphaMode::Mask(0.5)
        } else {
            AlphaMode::Opaque
        }
    }
}
