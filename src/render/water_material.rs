use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        error, AddAsset, App, Assets, Commands, Entity, FromWorld, HandleUntyped, Mesh, Msaa,
        Plugin, Query, Res, ResMut, Time, World,
    },
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType,
            BufferDescriptor, BufferUsages, FilterMode, PipelineCache, RenderPipelineDescriptor,
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderSize, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
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
            size: WaterUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("water_texture_index"),
        });

        app.add_asset::<WaterMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<WaterMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<WaterMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawWaterMaterial>()
                .init_resource::<WaterMaterialPipeline>()
                .insert_resource(WaterUniformMeta { buffer })
                .init_resource::<SpecializedMeshPipelines<WaterMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Extract, extract_water_uniform_data)
                .add_system_to_stage(RenderStage::Prepare, prepare_water_texture_index)
                .add_system_to_stage(RenderStage::Queue, queue_water_material_meshes);
        }
    }
}

#[derive(Clone, ShaderType)]
pub struct WaterUniformData {
    pub current_index: i32,
    pub next_index: i32,
    pub next_weight: f32,
}

fn extract_water_uniform_data(mut commands: Commands, time: Res<Time>) {
    let time = time.seconds_since_startup() * 10.0;
    let current_index = (time as i32) % 25;
    let next_index = (current_index + 1) % 25;
    let next_weight = time.fract() as f32;

    commands.insert_resource(WaterUniformData {
        current_index,
        next_index,
        next_weight,
    });
}

pub struct WaterUniformMeta {
    buffer: Buffer,
}

fn prepare_water_texture_index(
    water_uniform_data: Res<WaterUniformData>,
    water_uniform_meta: ResMut<WaterUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; WaterUniformData::SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(water_uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&water_uniform_meta.buffer, 0, buffer.as_ref());
}

pub struct WaterMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    pub sampler: Sampler,
}

impl SpecializedMeshPipeline for WaterMaterialPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

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

        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(descriptor)
    }
}

impl FromWorld for WaterMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                // Water Uniform Meta
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(WaterUniformData::min_size()),
                    },
                    count: None,
                },
            ],
            label: Some("water_material_layout"),
        });

        WaterMaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            vertex_shader: Some(WATER_MESH_MATERIAL_SHADER_HANDLE.typed()),
            fragment_shader: Some(WATER_MESH_MATERIAL_SHADER_HANDLE.typed()),
            sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..Default::default()
            }),
        }
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "e9e46dcc-94db-4b31-819f-d5ecffc732f0"]
pub struct WaterMaterial {
    pub water_texture_array: Handle<TextureArray>,
}

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
        SRes<WaterMaterialPipeline>,
        SRes<RenderAssets<TextureArray>>,
        SRes<WaterUniformMeta>,
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
            water_uniform_meta,
        ): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let water_texture_gpu_image = gpu_texture_arrays.get(&material.water_texture_array);
        if water_texture_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let water_texture_view = &water_texture_gpu_image.unwrap().texture_view;
        let water_texture_sampler = &material_pipeline.sampler;

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
                    resource: water_uniform_meta.buffer.as_entire_binding(),
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

pub struct SetWaterMaterialBindGroup<const I: usize>(PhantomData<WaterMaterial>);
impl<const I: usize> EntityRenderCommand for SetWaterMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<WaterMaterial>>,
        SQuery<Read<Handle<WaterMaterial>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

type DrawWaterMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetWaterMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    DrawMesh,
);

#[allow(clippy::too_many_arguments)]
pub fn queue_water_material_meshes(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<WaterMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WaterMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<WaterMaterial>>,
    material_meshes: Query<(&Handle<WaterMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    for (view, visible_entities, mut transparent_phase) in views.iter_mut() {
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .get_id::<DrawWaterMaterial>()
            .unwrap();

        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if render_materials.contains_key(material_handle) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | MeshPipelineKey::TRANSPARENT_MAIN_PASS
                                | msaa_key;

                        let pipeline_id = pipelines.specialize(
                            &mut pipeline_cache,
                            &material_pipeline,
                            mesh_key,
                            &mesh.layout,
                        );
                        let pipeline_id = match pipeline_id {
                            Ok(id) => id,
                            Err(err) => {
                                error!("{}", err);
                                continue;
                            }
                        };

                        // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                        // gives the z component of translation of the mesh in view space
                        let mesh_z = inverse_view_row_2.dot(mesh_uniform.transform.col(3));
                        transparent_phase.add(Transparent3d {
                            entity: *visible_entity,
                            draw_function: draw_transparent_pbr,
                            pipeline: pipeline_id,
                            // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                            // lowest sort key and getting closer should increase. As we have
                            // -z in front of the camera, the largest distance is -far with values increasing toward the
                            // camera. As such we can just use mesh_z as the distance
                            distance: mesh_z,
                        });
                    }
                }
            }
        }
    }
}
