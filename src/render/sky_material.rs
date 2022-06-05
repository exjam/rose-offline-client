use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    core_pipeline::core_3d::Opaque3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        error, AddAsset, App, Assets, Commands, Entity, FromWorld, HandleUntyped, Image, Mesh,
        Msaa, Plugin, Query, Res, ResMut, World,
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
            encase::{self, ShaderType, Size},
            AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, FilterMode, PipelineCache,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
        RenderApp, RenderStage,
    },
};

use crate::resources::{ZoneTime, ZoneTimeState};

pub const SKY_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xadc5cbbc7a53fe);

#[derive(Default)]
pub struct SkyMaterialPlugin;

impl Plugin for SkyMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            SKY_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/sky_material.wgsl")),
        );

        let render_device = app.world.resource::<RenderDevice>();
        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: SkyUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("sky_data_uniform_buffer"),
        });

        app.add_asset::<SkyMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<SkyMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<SkyMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawSkyMaterial>()
                .insert_resource(SkyUniformMeta {
                    buffer,
                    bind_group: None,
                })
                .init_resource::<SkyMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<SkyMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Extract, extract_sky_uniform_data)
                .add_system_to_stage(RenderStage::Prepare, prepare_sky_uniform_data)
                .add_system_to_stage(RenderStage::Queue, queue_sky_uniform_bind_group)
                .add_system_to_stage(RenderStage::Queue, queue_sky_material_meshes);
        }
    }
}

#[derive(Clone, ShaderType)]
pub struct SkyUniformData {
    pub day_weight: f32,
}

fn extract_sky_uniform_data(mut commands: Commands, zone_time: Res<ZoneTime>) {
    let day_weight = match zone_time.state {
        ZoneTimeState::Morning => zone_time.state_percent_complete,
        ZoneTimeState::Day => 1.0,
        ZoneTimeState::Evening => 1.0 - zone_time.state_percent_complete,
        ZoneTimeState::Night => 0.0,
    };

    commands.insert_resource(SkyUniformData { day_weight });
}

struct SkyUniformMeta {
    buffer: Buffer,
    bind_group: Option<BindGroup>,
}

fn prepare_sky_uniform_data(
    sky_uniform_data: Res<SkyUniformData>,
    sky_uniform_meta: ResMut<SkyUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; SkyUniformData::SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(sky_uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&sky_uniform_meta.buffer, 0, buffer.as_ref());
}

fn queue_sky_uniform_bind_group(
    render_device: Res<RenderDevice>,
    mut sky_uniform_meta: ResMut<SkyUniformMeta>,
    pipeline: Res<SkyMaterialPipeline>,
) {
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.sky_uniform_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: sky_uniform_meta.buffer.as_entire_binding(),
        }],
    });
    sky_uniform_meta.bind_group = Some(bind_group);
}

pub struct SkyMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub sky_uniform_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    pub sampler: Sampler,
}

impl SpecializedMeshPipeline for SkyMaterialPipeline {
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

        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.sky_uniform_layout.clone(),
        ]);

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(descriptor)
    }
}

impl FromWorld for SkyMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Texture 0
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
                // Texture 0 Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Texture 1
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Texture 1 Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("sky_material_layout"),
        });

        let sky_uniform_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(SkyUniformData::min_size()),
                    },
                    count: None,
                }],
                label: Some("sky_uniform_layout"),
            });

        SkyMaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            sky_uniform_layout,
            vertex_shader: Some(SKY_MATERIAL_SHADER_HANDLE.typed()),
            fragment_shader: Some(SKY_MATERIAL_SHADER_HANDLE.typed()),
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
#[uuid = "971a6c96-4516-4ea0-aeb6-349633e7934e"]
pub struct SkyMaterial {
    pub texture_day: Option<Handle<Image>>,
    pub texture_night: Option<Handle<Image>>,
}

/// The GPU representation of a [`SkyMaterial`].
#[derive(Debug, Clone)]
pub struct GpuSkyMaterial {
    pub bind_group: BindGroup,
    pub texture_day: Option<Handle<Image>>,
    pub texture_night: Option<Handle<Image>>,
}

impl RenderAsset for SkyMaterial {
    type ExtractedAsset = SkyMaterial;
    type PreparedAsset = GpuSkyMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<SkyMaterialPipeline>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (texture_day_view, _) = if let Some(result) = material_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.texture_day)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
        let texture_day_sampler = &material_pipeline.sampler;

        let (texture_night_view, _) = if let Some(result) = material_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_images, &material.texture_night)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };
        let texture_night_sampler = &material_pipeline.sampler;

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture_day_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(texture_day_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(texture_night_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(texture_night_sampler),
                },
            ],
            label: Some("sky_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuSkyMaterial {
            bind_group,
            texture_day: material.texture_day,
            texture_night: material.texture_night,
        })
    }
}

pub struct SetSkyMaterialBindGroup<const I: usize>(PhantomData<SkyMaterial>);
impl<const I: usize> EntityRenderCommand for SetSkyMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<SkyMaterial>>,
        SQuery<Read<Handle<SkyMaterial>>>,
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
    type Param = SRes<SkyUniformMeta>;

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

type DrawSkyMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetSkyMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetTimeBindGroup<3>,
    DrawMesh,
);

fn queue_sky_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    material_pipeline: Res<SkyMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<SkyMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<SkyMaterial>>,
    material_meshes: Query<(&Handle<SkyMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(&ExtractedView, &VisibleEntities, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_opaque = opaque_draw_functions
        .read()
        .get_id::<DrawSkyMaterial>()
        .unwrap();

    for (view, visible_entities, mut opaque_phase) in views.iter_mut() {
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let _inverse_view_row_2 = inverse_view_matrix.row(2);
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, _mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if render_materials.get(material_handle).is_some() {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
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

                        opaque_phase.add(Opaque3d {
                            entity: *visible_entity,
                            draw_function: draw_opaque,
                            pipeline: pipeline_id,
                            distance: -9999999999.0,
                        });
                    }
                }
            }
        }
    }
}
