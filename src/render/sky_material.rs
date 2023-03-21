use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    pbr::{
        extract_materials, prepare_materials, queue_material_meshes, queue_shadows, DrawMesh,
        DrawPrepass, ExtractedMaterials, MaterialPipeline, MeshPipeline, MeshPipelineKey,
        MeshUniform, PrepassPipelinePlugin, RenderLightSystems, RenderMaterials,
        SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup, Shadow,
    },
    prelude::{
        error, AddAsset, App, Assets, Commands, FromWorld, HandleUntyped, Image,
        IntoSystemAppConfig, IntoSystemConfig, Material, Mesh, Msaa, Plugin, Query, Res, ResMut,
        Resource, World,
    },
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{
            PrepareAssetError, PrepareAssetSet, RenderAsset, RenderAssetPlugin, RenderAssets,
        },
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase::{self, ShaderType},
            AddressMode, AsBindGroup, BindGroup, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
            BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
            CompareFunction, FilterMode, PipelineCache, RenderPipelineDescriptor, Sampler,
            SamplerBindingType, SamplerDescriptor, ShaderSize, ShaderStages,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
        Extract, ExtractSchedule, RenderApp, RenderSet,
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

        /*
        app.add_asset::<SkyMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<SkyMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<SkyMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawSkyMaterial>()
                .init_resource::<SkyUniformMeta>()
                .init_resource::<SkyMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<SkyMaterialPipeline>>()
                .add_system(extract_sky_uniform_data.in_schedule(ExtractSchedule))
                .add_system(prepare_sky_uniform_data.in_set(RenderSet::Prepare))
                .add_system(queue_sky_material_meshes.in_set(RenderSet::Queue));
        }
        */

        app.add_asset::<SkyMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<SkyMaterial>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<DrawFunctions<Shadow>>()
                .add_render_command::<Shadow, DrawPrepass<SkyMaterial>>()
                .add_render_command::<Transparent3d, DrawSkyMaterial>()
                .add_render_command::<Opaque3d, DrawSkyMaterial>()
                .add_render_command::<AlphaMask3d, DrawSkyMaterial>()
                .init_resource::<MaterialPipeline<SkyMaterial>>()
                .init_resource::<ExtractedMaterials<SkyMaterial>>()
                .init_resource::<RenderMaterials<SkyMaterial>>()
                .init_resource::<SpecializedMeshPipelines<MaterialPipeline<SkyMaterial>>>()
                .add_system(extract_materials::<SkyMaterial>.in_schedule(ExtractSchedule))
                .add_system(
                    prepare_materials::<SkyMaterial>
                        .in_set(RenderSet::Prepare)
                        .after(PrepareAssetSet::PreAssetPrepare),
                )
                .add_system(
                    queue_shadows::<SkyMaterial, DrawPrepass<SkyMaterial>>
                        .in_set(RenderLightSystems::QueueShadows),
                )
                .add_system(
                    queue_material_meshes::<SkyMaterial, DrawSkyMaterial>.in_set(RenderSet::Queue),
                );

            render_app
                .init_resource::<SkyUniformMeta>()
                .add_system(extract_sky_uniform_data.in_schedule(ExtractSchedule))
                .add_system(prepare_sky_uniform_data.in_set(RenderSet::Prepare));
        }

        // PrepassPipelinePlugin is required for shadow mapping and the optional PrepassPlugin
        app.add_plugin(PrepassPipelinePlugin::<SkyMaterial>::default());

        //if self.prepass_enabled {
        //    app.add_plugin(PrepassPlugin::<SkyMaterial>::default());
        //}
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct SkyUniformData {
    pub day_weight: f32,
}

fn extract_sky_uniform_data(mut commands: Commands, zone_time: Extract<Res<ZoneTime>>) {
    let day_weight = match zone_time.state {
        ZoneTimeState::Morning => zone_time.state_percent_complete,
        ZoneTimeState::Day => 1.0,
        ZoneTimeState::Evening => 1.0 - zone_time.state_percent_complete,
        ZoneTimeState::Night => 0.0,
    };

    commands.insert_resource(SkyUniformData { day_weight });
}

#[derive(Resource)]
struct SkyUniformMeta {
    buffer: Buffer,
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for SkyUniformMeta {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: SkyUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("sky_data_uniform_buffer"),
        });

        let bind_group_layout =
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

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        SkyUniformMeta {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

fn prepare_sky_uniform_data(
    sky_uniform_data: Res<SkyUniformData>,
    sky_uniform_meta: ResMut<SkyUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; SkyUniformData::SHADER_SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(sky_uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&sky_uniform_meta.buffer, 0, buffer.as_ref());
}

#[derive(Debug, Clone, TypeUuid, AsBindGroup)]
#[uuid = "971a6c96-4516-4ea0-aeb6-349633e7934e"]
pub struct SkyMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture_day: Option<Handle<Image>>,

    #[texture(2)]
    #[sampler(3)]
    pub texture_night: Option<Handle<Image>>,
}

#[derive(Clone)]
pub struct SkyMaterialPipelineData {
    pub sky_uniform_layout: BindGroupLayout,
}

impl FromWorld for SkyMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        Self {
            sky_uniform_layout: world.resource::<SkyUniformMeta>().bind_group_layout.clone(),
        }
    }
}

impl Material for SkyMaterial {
    type PipelineData = SkyMaterialPipelineData;

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        bevy::prelude::AlphaMode::Opaque
    }

    fn depth_bias(&self) -> f32 {
        9999999999.0
    }

    fn prepass_vertex_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn prepass_fragment_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;
        descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;

        descriptor
            .layout
            .insert(3, pipeline.data.sky_uniform_layout.clone());

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

struct SetTimeBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTimeBindGroup<I> {
    type Param = SRes<SkyUniformMeta>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &meta.into_inner().bind_group, &[]);

        RenderCommandResult::Success
    }
}

type DrawSkyMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<SkyMaterial, 1>,
    SetMeshBindGroup<2>,
    SetTimeBindGroup<3>,
    DrawMesh,
);
