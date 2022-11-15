use bevy::{
    app::prelude::*,
    asset::{Assets, Handle, HandleUntyped},
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    math::prelude::*,
    prelude::{Msaa, Shader},
    reflect::TypeUuid,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{ComputedVisibility, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract, RenderApp, RenderStage,
    },
};
use bytemuck::Pod;
use std::{collections::HashMap, num::NonZeroU64, ops::Range};

use crate::render::{DamageDigitMaterial, DamageDigitRenderData};

pub const DAMAGE_DIGIT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 39699708885);

pub struct DamageDigitRenderPlugin;

impl Plugin for DamageDigitRenderPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            DAMAGE_DIGIT_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/damage_digit.wgsl")),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_system_to_stage(RenderStage::Extract, extract_damage_digits)
            .add_system_to_stage(RenderStage::Prepare, prepare_damage_digits)
            .add_system_to_stage(RenderStage::Queue, queue_damage_digits)
            .init_resource::<DamageDigitPipeline>()
            .init_resource::<DamageDigitMeta>()
            .init_resource::<ExtractedDamageDigits>()
            .init_resource::<MaterialBindGroups>()
            .init_resource::<SpecializedRenderPipelines<DamageDigitPipeline>>()
            .add_render_command::<Transparent3d, DrawDamageDigit>();
    }
}

#[derive(Resource)]
struct DamageDigitPipeline {
    view_layout: BindGroupLayout,
    particle_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for DamageDigitPipeline {
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
                // Positions
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
                // UVs
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
    pub struct DamageDigitPipelineKey: u32 {
        const NONE                        = 0;
        const MSAA_RESERVED_BITS          = DamageDigitPipelineKey::MSAA_MASK_BITS << DamageDigitPipelineKey::MSAA_SHIFT_BITS;
    }
}

impl DamageDigitPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        DamageDigitPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for DamageDigitPipeline {
    type Key = DamageDigitPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: DAMAGE_DIGIT_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "vs_main".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: DAMAGE_DIGIT_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
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
            label: Some("damage_digit_render_pipeline".into()),
        }
    }
}

struct ExtractedDamageDigitRenderData {
    material: Handle<DamageDigitMaterial>,
    positions: Vec<Vec4>,
    sizes: Vec<Vec2>,
    uvs: Vec<Vec4>,
}

#[derive(Default, Component, Resource)]
struct ExtractedDamageDigits {
    particles: Vec<ExtractedDamageDigitRenderData>,
}

fn extract_damage_digits(
    mut extracted_damage_digits: ResMut<ExtractedDamageDigits>,
    materials: Extract<Res<Assets<DamageDigitMaterial>>>,
    images: Extract<Res<Assets<Image>>>,
    query: Extract<
        Query<(
            &ComputedVisibility,
            &DamageDigitRenderData,
            &Handle<DamageDigitMaterial>,
        )>,
    >,
) {
    extracted_damage_digits.particles.clear();
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

            extracted_damage_digits
                .particles
                .push(ExtractedDamageDigitRenderData {
                    material: material_handle.clone_weak(),
                    positions: particles.positions.clone(),
                    sizes: particles.sizes.clone(),
                    uvs: particles.uvs.clone(),
                });
        }
    }
}

#[derive(Resource)]
struct DamageDigitMeta {
    ranges: Vec<Range<u64>>,
    total_count: u64,
    view_bind_group: Option<BindGroup>,
    particle_bind_group: Option<BindGroup>,

    positions: BufferVec<Vec4>,
    sizes: BufferVec<Vec2>,
    uvs: BufferVec<Vec4>,
}

impl Default for DamageDigitMeta {
    fn default() -> Self {
        DamageDigitMeta {
            ranges: Vec::default(),
            total_count: 0,
            view_bind_group: None,
            particle_bind_group: None,

            positions: BufferVec::new(BufferUsages::STORAGE),
            sizes: BufferVec::new(BufferUsages::STORAGE),
            uvs: BufferVec::new(BufferUsages::STORAGE),
        }
    }
}

fn prepare_damage_digits(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut commands: Commands,
    mut particle_meta: ResMut<DamageDigitMeta>,
    mut extracted_damage_digits: ResMut<ExtractedDamageDigits>,
) {
    particle_meta.positions.clear();
    particle_meta.sizes.clear();
    particle_meta.uvs.clear();

    let mut total_count = 0;
    for particle in extracted_damage_digits.particles.iter() {
        total_count += particle.positions.len();
    }

    particle_meta.total_count = total_count as u64;
    particle_meta.ranges.clear();
    if total_count == 0 {
        return;
    }

    particle_meta.positions.reserve(total_count, &render_device);
    particle_meta.sizes.reserve(total_count, &render_device);
    particle_meta.uvs.reserve(total_count, &render_device);

    extracted_damage_digits
        .particles
        .sort_by(|a, b| a.material.cmp(&b.material));

    let mut start: u32 = 0;
    let mut end: u32 = 0;
    let mut current_batch: Option<Handle<DamageDigitMaterial>> = None;
    for particle in extracted_damage_digits.particles.iter() {
        if start != end {
            if let Some(current_batch_material) = &current_batch {
                if current_batch_material != &particle.material {
                    let current_batch_material = current_batch.take().unwrap();
                    commands.spawn(DamageDigitBatch {
                        range: start..end,
                        handle: current_batch_material,
                    });
                    current_batch = Some(particle.material.clone_weak());
                    start = end;
                }
            }
        } else {
            current_batch = Some(particle.material.clone_weak());
        }

        batch_copy(&particle.positions, &mut particle_meta.positions);
        batch_copy(&particle.sizes, &mut particle_meta.sizes);
        batch_copy(&particle.uvs, &mut particle_meta.uvs);
        end += particle.positions.len() as u32;
    }

    if start != end {
        if let Some(current_batch_material) = current_batch {
            commands.spawn(DamageDigitBatch {
                range: start..end,
                handle: current_batch_material,
            });
        }
    }

    particle_meta
        .positions
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .sizes
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .uvs
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
struct DamageDigitBatch {
    range: Range<u32>,
    handle: Handle<DamageDigitMaterial>,
}

#[derive(Default, Resource)]
struct MaterialBindGroups {
    values: HashMap<Handle<DamageDigitMaterial>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
fn queue_damage_digits(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<&mut RenderPhase<Transparent3d>>,
    render_device: Res<RenderDevice>,
    mut material_bind_groups: ResMut<MaterialBindGroups>,
    mut damage_digit_meta: ResMut<DamageDigitMeta>,
    view_uniforms: Res<ViewUniforms>,
    damage_digit_pipeline: Res<DamageDigitPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DamageDigitPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    damage_digit_batches: Query<(Entity, &DamageDigitBatch)>,
    render_materials: Res<RenderAssets<DamageDigitMaterial>>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
) {
    if view_uniforms.uniforms.is_empty() || damage_digit_meta.total_count == 0 {
        return;
    }
    let msaa_key = DamageDigitPipelineKey::from_msaa_samples(msaa.samples);

    if let Some(view_bindings) = view_uniforms.uniforms.binding() {
        damage_digit_meta.view_bind_group.get_or_insert_with(|| {
            render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_bindings,
                }],
                label: Some("damage_digit_view_bind_group"),
                layout: &damage_digit_pipeline.view_layout,
            })
        });
    }

    damage_digit_meta.particle_bind_group =
        Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bind_buffer(
                        &damage_digit_meta.positions,
                        damage_digit_meta.total_count,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bind_buffer(&damage_digit_meta.sizes, damage_digit_meta.total_count),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bind_buffer(&damage_digit_meta.uvs, damage_digit_meta.total_count),
                },
            ],
            label: Some("damage_digit_bind_group"),
            layout: &damage_digit_pipeline.particle_layout,
        }));

    let draw_particle_function = transparent_draw_functions
        .read()
        .get_id::<DrawDamageDigit>()
        .unwrap();
    for mut transparent_phase in views.iter_mut() {
        for (entity, batch) in damage_digit_batches.iter() {
            let gpu_material = render_materials
                .get(&batch.handle)
                .expect("Failed to get DamageDigitMaterial PreparedAsset");

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
                        label: Some("damage_digit_material_bind_group"),
                        layout: &damage_digit_pipeline.material_layout,
                    }),
                );
            }

            transparent_phase.add(Transparent3d {
                distance: 10.0,
                pipeline: pipelines.specialize(
                    &mut pipeline_cache,
                    &damage_digit_pipeline,
                    msaa_key,
                ),
                entity,
                draw_function: draw_particle_function,
            });
        }
    }
}

type DrawDamageDigit = (
    SetItemPipeline,
    SetDamageDigitViewBindGroup<0>,
    SetDamageDigitBindGroup<1>,
    SetDamageDigitMaterialBindGroup<2>,
    DrawDamageDigitBatch,
);

struct SetDamageDigitViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetDamageDigitViewBindGroup<I> {
    type Param = (SRes<DamageDigitMeta>, SQuery<Read<ViewUniformOffset>>);

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

struct SetDamageDigitBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetDamageDigitBindGroup<I> {
    type Param = SRes<DamageDigitMeta>;

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

struct SetDamageDigitMaterialBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetDamageDigitMaterialBindGroup<I> {
    type Param = (SRes<MaterialBindGroups>, SQuery<Read<DamageDigitBatch>>);

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

struct DrawDamageDigitBatch;
impl EntityRenderCommand for DrawDamageDigitBatch {
    type Param = SQuery<Read<DamageDigitBatch>>;

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
