use bevy::{
    app::prelude::*,
    asset::{Assets, Handle, HandleUntyped},
    core_pipeline::Transparent3d,
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    math::prelude::*,
    prelude::{Msaa, Shader},
    reflect::TypeUuid,
    render::{
        primitives::Aabb,
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{
            ComputedVisibility, ViewUniform, ViewUniformOffset, ViewUniforms, VisibilitySystems,
        },
        RenderApp, RenderStage, RenderWorld,
    },
    tasks::ComputeTaskPool,
};
use bytemuck::Pod;
use num_traits::FromPrimitive;
use std::{collections::HashMap, num::NonZeroU64, ops::Range};

use crate::render::{particle_render_data::ParticleRenderData, ParticleMaterial};

use super::particle_render_data::ParticleRenderBillboardType;

pub const PARTICLE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3032357527543835453);

pub struct ParticleRenderPlugin;

impl Plugin for ParticleRenderPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            PARTICLE_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/particle.wgsl")),
        );

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            compute_particles_aabb.label(VisibilitySystems::CalculateBounds),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_system_to_stage(RenderStage::Extract, extract_particles)
            .add_system_to_stage(RenderStage::Prepare, prepare_particles)
            .add_system_to_stage(RenderStage::Queue, queue_particles)
            .init_resource::<ParticlePipeline>()
            .init_resource::<ParticleMeta>()
            .init_resource::<ExtractedParticles>()
            .init_resource::<MaterialBindGroups>()
            .init_resource::<SpecializedRenderPipelines<ParticlePipeline>>()
            .add_render_command::<Transparent3d, DrawParticle>();
    }
}

struct ParticlePipeline {
    view_layout: BindGroupLayout,
    particle_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for ParticlePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(std::mem::size_of::<ViewUniform>() as u64),
                },
                count: None,
            }],
            label: None,
        });

        let particle_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // Positions/Rotations
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Vec4>() as u64),
                    },
                    count: None,
                },
                // Sizes
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Vec2>() as u64),
                    },
                    count: None,
                },
                // Colors
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Vec4>() as u64),
                    },
                    count: None,
                },
                // Textures
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Vec4>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // Base Color Texture
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
                // Base Color Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        Self {
            view_layout,
            particle_layout,
            material_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ParticlePipelineKey: u32 {
        const NONE                        = 0;
        const BLEND_OP_BITS               = ParticlePipelineKey::BLEND_OP_MASK_BITS << ParticlePipelineKey::BLEND_OP_SHIFT_BITS;
        const SRC_BLEND_FACTOR_BITS       = ParticlePipelineKey::BLEND_FACTOR_MASK_BITS << ParticlePipelineKey::SRC_BLEND_FACTOR_SHIFT_BITS;
        const DST_BLEND_FACTOR_BITS       = ParticlePipelineKey::BLEND_FACTOR_MASK_BITS << ParticlePipelineKey::DST_BLEND_FACTOR_SHIFT_BITS;
        const BILLBOARD_BITS              = ParticlePipelineKey::BILLBOARD_MASK_BITS << ParticlePipelineKey::BILLBOARD_SHIFT_BITS;
        const MSAA_RESERVED_BITS          = ParticlePipelineKey::MSAA_MASK_BITS << ParticlePipelineKey::MSAA_SHIFT_BITS;
    }
}

fn decode_blend_op(value: u32) -> BlendOperation {
    match value {
        1 => BlendOperation::Add,
        2 => BlendOperation::Subtract,
        3 => BlendOperation::ReverseSubtract,
        4 => BlendOperation::Min,
        5 => BlendOperation::Max,
        _ => BlendOperation::Add,
    }
}

fn decode_blend_factor(value: u32) -> BlendFactor {
    match value {
        1 => BlendFactor::Zero,
        2 => BlendFactor::One,
        3 => BlendFactor::Src,
        4 => BlendFactor::OneMinusSrc,
        5 => BlendFactor::SrcAlpha,
        6 => BlendFactor::OneMinusSrcAlpha,
        7 => BlendFactor::DstAlpha,
        8 => BlendFactor::OneMinusDstAlpha,
        9 => BlendFactor::Dst,
        10 => BlendFactor::OneMinusDst,
        11 => BlendFactor::SrcAlphaSaturated,
        _ => BlendFactor::Zero,
    }
}

impl ParticlePipelineKey {
    const BLEND_FACTOR_MASK_BITS: u32 = 0b1111;
    const BLEND_OP_MASK_BITS: u32 = 0b111;
    const SRC_BLEND_FACTOR_SHIFT_BITS: u32 = 8;
    const DST_BLEND_FACTOR_SHIFT_BITS: u32 = 8 + 4;
    const BLEND_OP_SHIFT_BITS: u32 = 8 + 8;

    const BILLBOARD_MASK_BITS: u32 = 0b11;
    const BILLBOARD_SHIFT_BITS: u32 = 16 + 3;

    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        ParticlePipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn from_blend(blend_op: u8, src_blend_factor: u8, dst_blend_factor: u8) -> Self {
        let blend_bits = (blend_op as u32) << Self::BLEND_OP_SHIFT_BITS
            | (src_blend_factor as u32) << Self::SRC_BLEND_FACTOR_SHIFT_BITS
            | (dst_blend_factor as u32) << Self::DST_BLEND_FACTOR_SHIFT_BITS;
        ParticlePipelineKey::from_bits(blend_bits).unwrap()
    }

    pub fn from_billboard(billboard_type: ParticleRenderBillboardType) -> Self {
        let billboard_bits = (billboard_type as u32) << Self::BILLBOARD_SHIFT_BITS;
        ParticlePipelineKey::from_bits(billboard_bits).unwrap()
    }

    pub fn billboard_type(&self) -> ParticleRenderBillboardType {
        FromPrimitive::from_u32(
            (self.bits >> Self::BILLBOARD_SHIFT_BITS) & Self::BILLBOARD_MASK_BITS,
        )
        .unwrap()
    }

    pub fn blend_op(&self) -> BlendOperation {
        decode_blend_op((self.bits >> Self::BLEND_OP_SHIFT_BITS) & Self::BLEND_OP_MASK_BITS)
    }

    pub fn src_blend_factor(&self) -> BlendFactor {
        decode_blend_factor(
            (self.bits >> Self::SRC_BLEND_FACTOR_SHIFT_BITS) & Self::BLEND_FACTOR_MASK_BITS,
        )
    }

    pub fn dst_blend_factor(&self) -> BlendFactor {
        decode_blend_factor(
            (self.bits >> Self::DST_BLEND_FACTOR_SHIFT_BITS) & Self::BLEND_FACTOR_MASK_BITS,
        )
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for ParticlePipeline {
    type Key = ParticlePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let src_factor = key.src_blend_factor();
        let dst_factor = key.dst_blend_factor();
        let operation = key.blend_op();

        let mut vs_shader_defs = Vec::new();
        match key.billboard_type() {
            ParticleRenderBillboardType::None => {}
            ParticleRenderBillboardType::YAxis => {
                vs_shader_defs.push("PARTICLE_BILLBOARD_Y_AXIS".to_string())
            }
            ParticleRenderBillboardType::Full => {
                vs_shader_defs.push("PARTICLE_BILLBOARD_FULL".to_string())
            }
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: PARTICLE_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vs_main".into(),
                buffers: vec![],
                shader_defs: vs_shader_defs,
            },
            fragment: Some(FragmentState {
                shader: PARTICLE_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor,
                            dst_factor,
                            operation,
                        },
                        alpha: BlendComponent {
                            src_factor,
                            dst_factor,
                            operation,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![
                self.view_layout.clone(),
                self.particle_layout.clone(),
                self.material_layout.clone(),
            ]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("particle_render_pipeline".into()),
        }
    }
}

fn compute_particles_aabb(
    compute_task_pool: Res<ComputeTaskPool>,
    mut query: Query<(&mut Aabb, &ParticleRenderData)>,
) {
    query.par_for_each_mut(&compute_task_pool, 8, |(mut aabb, particles)| {
        if let Some(bounding_box) = particles.compute_aabb() {
            *aabb = bounding_box;
        }
    });
}

struct ExtractedParticleRenderData {
    material: Handle<ParticleMaterial>,
    material_key: ParticlePipelineKey,

    positions: Vec<Vec4>,
    sizes: Vec<Vec2>,
    colors: Vec<Vec4>,
    textures: Vec<Vec4>,
}

#[derive(Default, Component)]
struct ExtractedParticles {
    particles: Vec<ExtractedParticleRenderData>,
}

fn extract_particles(
    mut render_world: ResMut<RenderWorld>,
    materials: Res<Assets<ParticleMaterial>>,
    images: Res<Assets<Image>>,
    query: Query<(
        &ComputedVisibility,
        &ParticleRenderData,
        &Handle<ParticleMaterial>,
    )>,
) {
    let mut extracted_particles = render_world
        .get_resource_mut::<ExtractedParticles>()
        .unwrap();
    extracted_particles.particles.clear();
    for (_visible, particles, material_handle) in query.iter() {
        /*
        // TODO: Fix aabb calculation so culling works correctly.
        if !visible.is_visible {
            continue;
        }
        */
        if let Some(material) = materials.get(material_handle) {
            if !images.contains(&material.texture) {
                continue;
            }

            extracted_particles
                .particles
                .push(ExtractedParticleRenderData {
                    material: material_handle.clone_weak(),
                    material_key: ParticlePipelineKey::from_billboard(particles.billboard_type)
                        | ParticlePipelineKey::from_blend(
                            particles.blend_op,
                            particles.src_blend_factor,
                            particles.dst_blend_factor,
                        ),
                    positions: particles.positions.clone(),
                    sizes: particles.sizes.clone(),
                    colors: particles.colors.clone(),
                    textures: particles.textures.clone(),
                });
        }
    }
}

struct ParticleMeta {
    ranges: Vec<Range<u64>>,
    total_count: u64,
    view_bind_group: Option<BindGroup>,
    particle_bind_group: Option<BindGroup>,

    positions: BufferVec<Vec4>,
    sizes: BufferVec<Vec2>,
    colors: BufferVec<Vec4>,
    textures: BufferVec<Vec4>,
}

impl Default for ParticleMeta {
    fn default() -> Self {
        ParticleMeta {
            ranges: Vec::default(),
            total_count: 0,
            view_bind_group: None,
            particle_bind_group: None,

            positions: BufferVec::new(BufferUsages::STORAGE),
            sizes: BufferVec::new(BufferUsages::STORAGE),
            colors: BufferVec::new(BufferUsages::STORAGE),
            textures: BufferVec::new(BufferUsages::STORAGE),
        }
    }
}

fn prepare_particles(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut commands: Commands,
    mut particle_meta: ResMut<ParticleMeta>,
    mut extracted_particles: ResMut<ExtractedParticles>,
) {
    particle_meta.positions.clear();
    particle_meta.sizes.clear();
    particle_meta.colors.clear();
    particle_meta.textures.clear();

    let mut total_count = 0;
    for particle in extracted_particles.particles.iter() {
        total_count += particle.positions.len();
    }

    particle_meta.total_count = total_count as u64;
    particle_meta.ranges.clear();
    if total_count == 0 {
        return;
    }

    particle_meta.positions.reserve(total_count, &render_device);
    particle_meta.sizes.reserve(total_count, &render_device);
    particle_meta.colors.reserve(total_count, &render_device);
    particle_meta.textures.reserve(total_count, &render_device);

    extracted_particles
        .particles
        .sort_by(|a, b| (a.material_key, &a.material).cmp(&(b.material_key, &b.material)));

    let mut start: u32 = 0;
    let mut end: u32 = 0;
    let mut current_batch: Option<(ParticlePipelineKey, Handle<ParticleMaterial>)> = None;
    for particle in extracted_particles.particles.iter() {
        if start != end {
            if let Some((current_batch_key, current_batch_material)) = &current_batch {
                if current_batch_key != &particle.material_key
                    || current_batch_material != &particle.material
                {
                    let (current_batch_key, current_batch_material) = current_batch.take().unwrap();
                    commands.spawn_bundle((ParticleBatch {
                        range: start..end,
                        handle: current_batch_material,
                        material_key: current_batch_key,
                    },));
                    current_batch = Some((particle.material_key, particle.material.clone_weak()));
                    start = end;
                }
            }
        } else {
            current_batch = Some((particle.material_key, particle.material.clone_weak()));
        }

        batch_copy(&particle.positions, &mut particle_meta.positions);
        batch_copy(&particle.sizes, &mut particle_meta.sizes);
        batch_copy(&particle.colors, &mut particle_meta.colors);
        batch_copy(&particle.textures, &mut particle_meta.textures);
        end += particle.positions.len() as u32;
    }

    if start != end {
        if let Some((current_batch_key, current_batch_material)) = current_batch {
            commands.spawn_bundle((ParticleBatch {
                range: start..end,
                handle: current_batch_material,
                material_key: current_batch_key,
            },));
        }
    }

    particle_meta
        .positions
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .sizes
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .colors
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .textures
        .write_buffer(&render_device, &render_queue);
}

fn batch_copy<T: Pod>(src: &[T], dst: &mut BufferVec<T>) {
    for item in src.iter() {
        dst.push(*item);
    }
}

fn bind_buffer<T: Pod>(buffer: &BufferVec<T>, count: u64) -> BindingResource {
    BindingResource::Buffer(BufferBinding {
        buffer: buffer.buffer().expect("missing buffer"),
        offset: 0,
        size: Some(NonZeroU64::new(std::mem::size_of::<T>() as u64 * count).unwrap()),
    })
}

#[derive(Component)]
struct ParticleBatch {
    range: Range<u32>,
    handle: Handle<ParticleMaterial>,
    material_key: ParticlePipelineKey,
}

#[derive(Default)]
struct MaterialBindGroups {
    values: HashMap<Handle<ParticleMaterial>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
fn queue_particles(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<&mut RenderPhase<Transparent3d>>,
    render_device: Res<RenderDevice>,
    mut material_bind_groups: ResMut<MaterialBindGroups>,
    mut particle_meta: ResMut<ParticleMeta>,
    view_uniforms: Res<ViewUniforms>,
    particle_pipeline: Res<ParticlePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ParticlePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    particle_batches: Query<(Entity, &ParticleBatch)>,
    render_materials: Res<RenderAssets<ParticleMaterial>>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
) {
    if view_uniforms.uniforms.is_empty() || particle_meta.total_count == 0 {
        return;
    }
    let msaa_key = ParticlePipelineKey::from_msaa_samples(msaa.samples);

    if let Some(view_bindings) = view_uniforms.uniforms.binding() {
        particle_meta.view_bind_group.get_or_insert_with(|| {
            render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_bindings,
                }],
                label: Some("particle_view_bind_group"),
                layout: &particle_pipeline.view_layout,
            })
        });
    }

    // TODO(james7132): Find a way to cache this.
    particle_meta.particle_bind_group =
        Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bind_buffer(&particle_meta.positions, particle_meta.total_count),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bind_buffer(&particle_meta.sizes, particle_meta.total_count),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bind_buffer(&particle_meta.colors, particle_meta.total_count),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: bind_buffer(&particle_meta.textures, particle_meta.total_count),
                },
            ],
            label: Some("particle_particle_bind_group"),
            layout: &particle_pipeline.particle_layout,
        }));

    let draw_particle_function = transparent_draw_functions
        .read()
        .get_id::<DrawParticle>()
        .unwrap();
    for mut transparent_phase in views.iter_mut() {
        for (entity, batch) in particle_batches.iter() {
            let gpu_material = render_materials
                .get(&batch.handle)
                .expect("Failed to get ParticleMaterial PreparedAsset");

            if let Some(gpu_image) = gpu_images.get(&gpu_material.texture) {
                material_bind_groups.values.insert(
                    batch.handle.clone_weak(),
                    render_device.create_bind_group(&BindGroupDescriptor {
                        entries: &[
                            BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::TextureView(&gpu_image.texture_view),
                            },
                            BindGroupEntry {
                                binding: 1,
                                resource: BindingResource::Sampler(&gpu_image.sampler),
                            },
                        ],
                        label: Some("particle_material_bind_group"),
                        layout: &particle_pipeline.material_layout,
                    }),
                );
            }

            transparent_phase.add(Transparent3d {
                // TODO(james7132): properly compute this
                distance: 10.0,
                pipeline: pipelines.specialize(
                    &mut pipeline_cache,
                    &particle_pipeline,
                    msaa_key | batch.material_key,
                ),
                entity,
                draw_function: draw_particle_function,
            });
        }
    }
}

type DrawParticle = (
    SetItemPipeline,
    SetParticleViewBindGroup<0>,
    SetParticleBindGroup<1>,
    SetParticleMaterialBindGroup<2>,
    DrawParticleBatch,
);

struct SetParticleViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetParticleViewBindGroup<I> {
    type Param = (SRes<ParticleMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (particle_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            particle_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

struct SetParticleBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetParticleBindGroup<I> {
    type Param = SRes<ParticleMeta>;

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        particle_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            particle_meta
                .into_inner()
                .particle_bind_group
                .as_ref()
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

struct SetParticleMaterialBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetParticleMaterialBindGroup<I> {
    type Param = (SRes<MaterialBindGroups>, SQuery<Read<ParticleBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (material_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batch = query_batch.get(item).unwrap();
        pass.set_bind_group(
            I,
            material_bind_groups
                .into_inner()
                .values
                .get(&batch.handle)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

struct DrawParticleBatch;
impl EntityRenderCommand for DrawParticleBatch {
    type Param = SQuery<Read<ParticleBatch>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        query_batch: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batch = query_batch.get(item).unwrap();
        let vertex_range = (batch.range.start * 6)..(batch.range.end * 6);
        pass.draw(vertex_range, 0..1);
        RenderCommandResult::Success
    }
}
