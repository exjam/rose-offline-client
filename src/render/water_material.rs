use bevy::{
    asset::Handle,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{AlphaMode, MaterialPipeline, MaterialPlugin, SpecializedMaterial},
    prelude::{App, AssetServer, Assets, Commands, HandleUntyped, Mesh, Plugin, Res, ResMut, Time},
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
            BlendOperation, BlendState, Buffer, BufferBindingType, BufferDescriptor, BufferSize,
            BufferUsages, FilterMode, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, SpecializedMeshPipelineError, TextureSampleType,
            TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
};

use crate::render::TextureArray;

pub const WATER_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x333959e64b35d5d9);

#[derive(Default)]
pub struct WaterMaterialPlugin;

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            WATER_MESH_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/water_material.wgsl")),
        );

        let render_device = app.world.resource::<RenderDevice>();
        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: std::mem::size_of::<i32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("water_texture_index"),
        });
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            label: Some("water_sampler"),
            ..Default::default()
        });

        app.add_plugin(MaterialPlugin::<WaterMaterial>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(WaterMaterialSampler { sampler })
                .insert_resource(WaterTextureIndex { buffer })
                .add_system_to_stage(RenderStage::Extract, extract_time)
                .add_system_to_stage(RenderStage::Prepare, prepare_water_texture_index);
        }
    }
}

#[derive(Default)]
struct ExtractedTime {
    seconds_since_startup: f64,
}

fn extract_time(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(ExtractedTime {
        seconds_since_startup: time.seconds_since_startup(),
    });
}

pub struct WaterMaterialSampler {
    sampler: Sampler,
}

pub struct WaterTextureIndex {
    buffer: Buffer,
}

fn prepare_water_texture_index(
    time: Res<ExtractedTime>,
    water_texture_index: ResMut<WaterTextureIndex>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &water_texture_index.buffer,
        0,
        bevy::core::cast_slice(&[(time.seconds_since_startup * 10.0) as i32 % 25]),
    );
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "e9e46dcc-94db-4b31-819f-d5ecffc732f0"]
pub struct WaterMaterial {
    pub water_texture_array: Handle<TextureArray>,
}

/// The GPU representation of a [`WaterMaterial`].
#[derive(Debug, Clone)]
pub struct GpuWaterMaterial {
    pub bind_group: BindGroup,
    pub water_texture_array: Handle<TextureArray>,
}

impl RenderAsset for WaterMaterial {
    type ExtractedAsset = WaterMaterial;
    type PreparedAsset = GpuWaterMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<WaterMaterial>>,
        SRes<RenderAssets<TextureArray>>,
        SRes<WaterMaterialSampler>,
        SRes<WaterTextureIndex>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (
            render_device,
            material_pipeline,
            gpu_texture_arrays,
            water_material_sampler,
            water_texture_index,
        ): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let water_texture_gpu_image = gpu_texture_arrays.get(&material.water_texture_array);
        if water_texture_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let water_texture_view = &water_texture_gpu_image.unwrap().texture_view;
        let water_texture_sampler = &water_material_sampler.sampler;

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                // Water Texture Array
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(water_texture_view),
                },
                // Water Texture Sampler
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(water_texture_sampler),
                },
                // Water Texture Index
                BindGroupEntry {
                    binding: 2,
                    resource: water_texture_index.buffer.as_entire_binding(),
                },
            ],
            label: Some("water_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuWaterMaterial {
            bind_group,
            water_texture_array: material.water_texture_array,
        })
    }
}

impl SpecializedMaterial for WaterMaterial {
    type Key = ();

    fn key(_render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {}

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        descriptor.fragment.as_mut().unwrap().targets[0].blend = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        });

        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        Ok(())
    }

    fn vertex_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(WATER_MESH_MATERIAL_SHADER_HANDLE.typed())
    }

    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(WATER_MESH_MATERIAL_SHADER_HANDLE.typed())
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
                // Water Texture Array
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Water Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Water Texture Index
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        min_binding_size: BufferSize::new(std::mem::size_of::<i32>() as u64),
                        has_dynamic_offset: false,
                    },
                    count: None,
                },
            ],
            label: Some("water_material_layout"),
        })
    }

    #[inline]
    fn alpha_mode(_render_asset: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Blend
    }
}
