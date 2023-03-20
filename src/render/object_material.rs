use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::Vec2,
    pbr::{
        AlphaMode, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        error, AddAsset, App, AssetServer, Assets, Component, FromWorld, HandleUntyped,
        IntoSystemConfig, Mesh, Msaa, Plugin, Query, Res, ResMut, Resource, Vec3, With, World,
    },
    reflect::{FromReflect, Reflect, TypeUuid},
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase::{self, ShaderType},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferUsages, CompareFunction, PipelineCache,
            RenderPipelineDescriptor, SamplerBindingType, ShaderDefVal, ShaderSize, ShaderStages,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::Image,
        view::{ExtractedView, VisibleEntities},
        RenderApp, RenderSet,
    },
};

use rose_file_readers::{ZscMaterialBlend, ZscMaterialGlow};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};

pub const OBJECT_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xb7ebbc00ea16d3c7);

#[derive(Default)]
pub struct ObjectMaterialPlugin;

impl Plugin for ObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            OBJECT_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/object_material.wgsl")),
        );

        app.add_asset::<ObjectMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<ObjectMaterial>>::extract_visible())
            .add_plugin(ExtractComponentPlugin::<ObjectMaterialClipFace>::extract_visible())
            .add_plugin(RenderAssetPlugin::<ObjectMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawObjectMaterial>()
                .add_render_command::<Opaque3d, DrawObjectMaterial>()
                .add_render_command::<AlphaMask3d, DrawObjectMaterial>()
                .init_resource::<ObjectMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<ObjectMaterialPipeline>>()
                .add_system(queue_object_material_meshes.in_set(RenderSet::Queue));
        }
    }
}

#[derive(Copy, Clone, Component)]
pub enum ObjectMaterialClipFace {
    First(u32),
    Last(u32),
}

impl ExtractComponent for ObjectMaterialClipFace {
    type Query = &'static Self;
    type Filter = With<Handle<ObjectMaterial>>;
    type Out = ObjectMaterialClipFace;

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self> {
        Some(*item)
    }
}

#[derive(Resource)]
pub struct ObjectMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub zone_lighting_layout: BindGroupLayout,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
    pub specular_texture: Option<Handle<Image>>,
}

impl SpecializedMeshPipeline for ObjectMaterialPipeline {
    type Key = (MeshPipelineKey, ObjectMaterialKey);

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.0, layout)?;
        descriptor.vertex.shader = self.vertex_shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.fragment_shader.clone();

        // MeshPipeline::specialize's current implementation guarantees that the returned
        // specialized descriptor has a populated layout
        descriptor.layout.insert(1, self.material_layout.clone());
        descriptor
            .layout
            .insert(3, self.zone_lighting_layout.clone());

        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];

        if key.1.has_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("HAS_OBJECT_LIGHTMAP".into(), true));
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(ShaderDefVal::Bool("HAS_OBJECT_LIGHTMAP".into(), true));

            vertex_attributes.push(MESH_ATTRIBUTE_UV_1.at_shader_location(3));
        } else {
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(ShaderDefVal::Bool("ZONE_LIGHTING_CHARACTER".into(), true));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("SKINNED".into(), true));
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(ShaderDefVal::Bool("SKINNED".into(), true));

            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
        } else if key.1.skinned {
            panic!("strange");
        }

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];

        if key.1.two_sided {
            descriptor.primitive.cull_mode = None;
        }

        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = key.1.z_write_enabled;

        if !key.1.z_test_enabled {
            descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;
        }

        descriptor.fragment.as_mut().unwrap().targets[0]
            .as_mut()
            .unwrap()
            .blend = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        });

        Ok(descriptor)
    }
}

impl FromWorld for ObjectMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let specular_texture = asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS");

        let render_device = world.resource::<RenderDevice>();

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Uniform data
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ObjectMaterialUniformData::min_size()),
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
                // Specular Texture
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Specular Texture Sampler
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("object_material_layout"),
        });

        ObjectMaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
            vertex_shader: OBJECT_MATERIAL_SHADER_HANDLE.typed(),
            fragment_shader: OBJECT_MATERIAL_SHADER_HANDLE.typed(),
            specular_texture: Some(specular_texture),
        }
    }
}

pub struct SetObjectMaterialBindGroup<const I: usize>(PhantomData<ObjectMaterial>);
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetObjectMaterialBindGroup<I> {
    type Param = SRes<RenderAssets<ObjectMaterial>>;
    type ItemWorldQuery = Read<Handle<ObjectMaterial>>;
    type ViewWorldQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'_, Self::ViewWorldQuery>,
        material_handle: ROQueryItem<'_, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawObjectMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawObjectMesh {
    type Param = SRes<RenderAssets<Mesh>>;
    type ItemWorldQuery = (Read<Handle<Mesh>>, Option<Read<ObjectMaterialClipFace>>);
    type ViewWorldQuery = ();

    #[inline]
    fn render<'w>(
        _: &P,
        _: ROQueryItem<'_, Self::ViewWorldQuery>,
        (mesh_handle, clip_face): ROQueryItem<'_, Self::ItemWorldQuery>,
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (start_index_offset, end_index_offset) = if let Some(clip_face) = clip_face {
            match clip_face {
                ObjectMaterialClipFace::First(num_faces) => (num_faces * 3, 0),
                ObjectMaterialClipFace::Last(num_faces) => (0, num_faces * 3),
            }
        } else {
            (0, 0)
        };

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    let start_index = start_index_offset;
                    let end_index = *count - end_index_offset;
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(start_index..end_index, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    let start_vertex = start_index_offset;
                    let end_vertex = *vertex_count - end_index_offset;
                    pass.draw(start_vertex..end_vertex, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

type DrawObjectMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetObjectMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawObjectMesh,
);

// NOTE: These must match the bit flags in shaders/object_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ObjectMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const ALPHA_MODE_BLEND           = (1 << 2);
        const HAS_ALPHA_VALUE            = (1 << 3);
        const SPECULAR                   = (1 << 4);
        const NONE                       = 0;
    }
}

#[derive(Clone, ShaderType)]
pub struct ObjectMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

#[derive(Copy, Clone, Debug, Default, Reflect)]
pub enum ObjectMaterialBlend {
    #[default]
    Normal,
    Lighten,
}

impl From<ZscMaterialBlend> for ObjectMaterialBlend {
    fn from(zsc: ZscMaterialBlend) -> Self {
        match zsc {
            ZscMaterialBlend::Normal => ObjectMaterialBlend::Normal,
            ZscMaterialBlend::Lighten => ObjectMaterialBlend::Lighten,
        }
    }
}

#[derive(Copy, Clone, Debug, Reflect, FromReflect)]
pub enum ObjectMaterialGlow {
    Simple(Vec3),
    Light(Vec3),
    Texture(Vec3),
    TextureLight(Vec3),
    Alpha(Vec3),
}

impl From<ZscMaterialGlow> for ObjectMaterialGlow {
    fn from(zsc: ZscMaterialGlow) -> Self {
        match zsc {
            ZscMaterialGlow::Simple(value) => {
                ObjectMaterialGlow::Simple(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Light(value) => {
                ObjectMaterialGlow::Light(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Texture(value) => {
                ObjectMaterialGlow::Texture(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::TextureLight(value) => {
                ObjectMaterialGlow::TextureLight(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Alpha(value) => {
                ObjectMaterialGlow::Alpha(Vec3::new(value.x, value.y, value.z))
            }
        }
    }
}

#[derive(Debug, Clone, TypeUuid, Reflect)]
#[uuid = "62a496fa-33e8-41a8-9a44-237d70214227"]
pub struct ObjectMaterial {
    pub base_texture: Option<Handle<Image>>,

    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub specular_enabled: bool,
    pub skinned: bool,
    pub blend: ObjectMaterialBlend,
    pub glow: Option<ObjectMaterialGlow>,

    // lightmap texture, uv offset, uv scale
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl Default for ObjectMaterial {
    fn default() -> Self {
        Self {
            base_texture: None,
            alpha_value: None,
            alpha_enabled: false,
            alpha_test: None,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
            specular_enabled: false,
            skinned: false,
            blend: ObjectMaterialBlend::Normal,
            glow: None,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuObjectMaterial {
    pub bind_group: BindGroup,

    pub uniform_buffer: Buffer,
    pub base_texture: Option<Handle<Image>>,
    pub lightmap_texture: Option<Handle<Image>>,

    pub flags: ObjectMaterialFlags,
    pub alpha_mode: AlphaMode,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,

    pub skinned: bool,
}

impl RenderAsset for ObjectMaterial {
    type ExtractedAsset = ObjectMaterial;
    type PreparedAsset = GpuObjectMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<ObjectMaterialPipeline>,
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
            .get_image_texture(gpu_images, &material.base_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (lightmap_texture_view, lightmap_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, &material.lightmap_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (specular_texture_view, specular_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, &material_pipeline.specular_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;
        let mut alpha_mode = AlphaMode::Opaque;

        if material.specular_enabled {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_mode = AlphaMode::Opaque;
            alpha_cutoff = 1.0;
        } else {
            if material.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;
                alpha_mode = AlphaMode::Blend;

                if let Some(alpha_ref) = material.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                    alpha_mode = AlphaMode::Mask(alpha_cutoff);
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = material.alpha_value {
                if material_alpha_value == 1.0 {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
                    alpha_mode = AlphaMode::Opaque;
                } else {
                    flags |= ObjectMaterialFlags::HAS_ALPHA_VALUE;
                    alpha_mode = AlphaMode::Blend;
                    alpha_value = material_alpha_value;
                }
            }
        }

        let value = ObjectMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff,
            alpha_value,
            lightmap_uv_offset: material.lightmap_uv_offset,
            lightmap_uv_scale: material.lightmap_uv_scale,
        };

        let byte_buffer = [0u8; ObjectMaterialUniformData::SHADER_SIZE.get() as usize];
        let mut buffer = encase::UniformBuffer::new(byte_buffer);
        buffer.write(&value).unwrap();

        let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("object_material_uniform_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: buffer.as_ref(),
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
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(specular_texture_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Sampler(specular_texture_sampler),
                },
            ],
            label: Some("object_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuObjectMaterial {
            bind_group,
            uniform_buffer,
            base_texture: material.base_texture,
            lightmap_texture: material.lightmap_texture,
            skinned: material.skinned,
            flags,
            alpha_mode,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    has_lightmap: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
    skinned: bool,
}

impl From<&GpuObjectMaterial> for ObjectMaterialKey {
    fn from(material: &GpuObjectMaterial) -> Self {
        ObjectMaterialKey {
            has_lightmap: material.lightmap_texture.is_some(),
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            skinned: material.skinned,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_object_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<ObjectMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<ObjectMaterialPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<ObjectMaterial>>,
    material_meshes: Query<(&Handle<ObjectMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    for (view, visible_entities, mut opaque_phase, mut alpha_mask_phase, mut transparent_phase) in
        views.iter_mut()
    {
        let draw_opaque_pbr = opaque_draw_functions
            .read()
            .get_id::<DrawObjectMaterial>()
            .unwrap();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions
            .read()
            .get_id::<DrawObjectMaterial>()
            .unwrap();
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .get_id::<DrawObjectMaterial>()
            .unwrap();

        let rangefinder = view.rangefinder3d();
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if let Some(material) = render_materials.get(material_handle) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mut mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | view_key;
                        let alpha_mode = material.alpha_mode;
                        if let AlphaMode::Blend = alpha_mode {
                            mesh_key |= MeshPipelineKey::BLEND_ALPHA;
                        }

                        let pipeline_id = pipelines.specialize(
                            &pipeline_cache,
                            &material_pipeline,
                            (mesh_key, material.into()),
                            &mesh.layout,
                        );
                        let pipeline_id = match pipeline_id {
                            Ok(id) => id,
                            Err(err) => {
                                error!("{}", err);
                                continue;
                            }
                        };

                        let distance = rangefinder.distance(&mesh_uniform.transform);
                        match alpha_mode {
                            AlphaMode::Opaque => {
                                opaque_phase.add(Opaque3d {
                                    entity: *visible_entity,
                                    draw_function: draw_opaque_pbr,
                                    pipeline: pipeline_id,
                                    distance,
                                });
                            }
                            AlphaMode::Mask(_) => {
                                alpha_mask_phase.add(AlphaMask3d {
                                    entity: *visible_entity,
                                    draw_function: draw_alpha_mask_pbr,
                                    pipeline: pipeline_id,
                                    distance,
                                });
                            }
                            AlphaMode::Blend
                            | AlphaMode::Premultiplied
                            | AlphaMode::Add
                            | AlphaMode::Multiply => {
                                transparent_phase.add(Transparent3d {
                                    entity: *visible_entity,
                                    draw_function: draw_transparent_pbr,
                                    pipeline: pipeline_id,
                                    distance,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}
