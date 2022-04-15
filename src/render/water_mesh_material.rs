use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    core::Time,
    core_pipeline::Transparent3d,
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
        Plugin, Query, Res, ResMut, World,
    },
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            std140::AsStd140, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer,
            BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, FilterMode,
            PipelineCache, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines, TextureSampleType, TextureViewDimension,
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
pub struct WaterMeshMaterialPlugin;

impl Plugin for WaterMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            WATER_MESH_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/water_mesh_material.wgsl")),
        );

        let render_device = app.world.resource::<RenderDevice>();
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("time uniform buffer"),
            size: std::mem::size_of::<i32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        app.add_asset::<WaterMeshMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<WaterMeshMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<WaterMeshMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawWaterMaterial>()
                .insert_resource(TimeMeta {
                    buffer,
                    bind_group: None,
                })
                .init_resource::<WaterMeshMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<WaterMeshMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Extract, extract_time)
                .add_system_to_stage(RenderStage::Prepare, prepare_time)
                .add_system_to_stage(RenderStage::Queue, queue_time_bind_group)
                .add_system_to_stage(RenderStage::Queue, queue_water_mesh_material_meshes);
        }
    }
}

#[derive(Default)]
struct ExtractedTime {
    seconds_since_startup: f64,
}

// extract the passed time into a resource in the render world
fn extract_time(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(ExtractedTime {
        seconds_since_startup: time.seconds_since_startup(),
    });
}

struct TimeMeta {
    buffer: Buffer,
    bind_group: Option<BindGroup>,
}

// write the extracted time into the corresponding uniform buffer
fn prepare_time(
    time: Res<ExtractedTime>,
    time_meta: ResMut<TimeMeta>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &time_meta.buffer,
        0,
        bevy::core::cast_slice(&[(time.seconds_since_startup * 10.0) as i32 % 25]),
    );
}

// create a bind group for the time uniform buffer
fn queue_time_bind_group(
    render_device: Res<RenderDevice>,
    mut time_meta: ResMut<TimeMeta>,
    pipeline: Res<WaterMeshMaterialPipeline>,
) {
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.time_uniform_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: time_meta.buffer.as_entire_binding(),
        }],
    });
    time_meta.bind_group = Some(bind_group);
}

pub struct WaterMeshMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub time_uniform_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    pub sampler: Sampler,
}

impl SpecializedMeshPipeline for WaterMeshMaterialPipeline {
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
        }
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.time_uniform_layout.clone(),
        ]);

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(descriptor)
    }
}

impl FromWorld for WaterMeshMaterialPipeline {
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
            ],
            label: Some("water_mesh_material_layout"),
        });

        let time_uniform_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("time bind group"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<i32>() as u64),
                    },
                    count: None,
                }],
            });

        WaterMeshMaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            time_uniform_layout,
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

#[derive(Clone, AsStd140)]
pub struct WaterMeshMaterialUniformData {
    pub texture_index: i32,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "e9e46dcc-94db-4b31-819f-d5ecffc732f0"]
pub struct WaterMeshMaterial {
    pub water_texture_array: Handle<TextureArray>,
}

/// The GPU representation of a [`WaterMeshMaterial`].
#[derive(Debug, Clone)]
pub struct GpuWaterMeshMaterial {
    pub bind_group: BindGroup,
    pub water_texture_array: Handle<TextureArray>,
}

impl RenderAsset for WaterMeshMaterial {
    type ExtractedAsset = WaterMeshMaterial;
    type PreparedAsset = GpuWaterMeshMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<WaterMeshMaterialPipeline>,
        SRes<RenderAssets<TextureArray>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_texture_arrays): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let water_texture_gpu_image = gpu_texture_arrays.get(&material.water_texture_array);
        if water_texture_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let water_texture_view = &water_texture_gpu_image.unwrap().texture_view;
        let water_texture_sampler = &material_pipeline.sampler;

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(water_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(water_texture_sampler),
                },
            ],
            label: Some("water_mesh_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuWaterMeshMaterial {
            bind_group,
            water_texture_array: material.water_texture_array,
        })
    }
}

pub struct SetWaterMaterialBindGroup<const I: usize>(PhantomData<WaterMeshMaterial>);
impl<const I: usize> EntityRenderCommand for SetWaterMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<WaterMeshMaterial>>,
        SQuery<Read<Handle<WaterMeshMaterial>>>,
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

struct SetTimeBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTimeBindGroup<I> {
    type Param = SRes<TimeMeta>;

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        time_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let time_bind_group = time_meta.into_inner().bind_group.as_ref().unwrap();
        pass.set_bind_group(I, time_bind_group, &[]);

        RenderCommandResult::Success
    }
}

type DrawWaterMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetWaterMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetTimeBindGroup<3>,
    DrawMesh,
);

fn queue_water_mesh_material_meshes(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<WaterMeshMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WaterMeshMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<WaterMeshMaterial>>,
    material_meshes: Query<(&Handle<WaterMeshMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    let draw_transparent = transparent_draw_functions
        .read()
        .get_id::<DrawWaterMaterial>()
        .unwrap();

    for (view, visible_entities, mut transparent_phase) in views.iter_mut() {
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if render_materials.get(material_handle).is_some() {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | msaa_key
                                | MeshPipelineKey::TRANSPARENT_MAIN_PASS;

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
                            draw_function: draw_transparent,
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
