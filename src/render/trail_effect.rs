use std::collections::VecDeque;

use bevy::{
    app::prelude::*,
    asset::{load_internal_asset, Assets, Handle, HandleUntyped},
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        prelude::*,
        query::ROQueryItem,
        system::{lifetimeless::*, SystemParamItem},
    },
    math::prelude::*,
    pbr::MeshPipelineKey,
    prelude::{Color, GlobalTransform, Msaa, Shader},
    reflect::TypeUuid,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{
            ComputedVisibility, ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset,
            ViewUniforms,
        },
        Extract, ExtractSchedule, RenderApp, RenderSet,
    },
    time::Time,
};
use bytemuck::{Pod, Zeroable};
use std::{collections::HashMap, ops::Range};

use crate::resources::RenderConfiguration;

pub const TRAIL_EFFECT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3042057527543835453);

pub struct TrailEffectRenderPlugin;

impl Plugin for TrailEffectRenderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TRAIL_EFFECT_SHADER_HANDLE,
            "shaders/trail_effect.wgsl",
            Shader::from_wgsl
        );

        app.add_systems(
            (initialise_trail_effects, update_trail_effects).in_base_set(CoreSet::PostUpdate),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_system(extract_trail_effects.in_schedule(ExtractSchedule))
            .add_system(prepare_trail_effects.in_set(RenderSet::Prepare))
            .add_system(queue_trail_effects.in_set(RenderSet::Queue))
            .init_resource::<TrailEffectPipeline>()
            .init_resource::<TrailEffectMeta>()
            .init_resource::<ExtractedTrailEffects>()
            .init_resource::<MaterialBindGroups>()
            .init_resource::<SpecializedRenderPipelines<TrailEffectPipeline>>()
            .add_render_command::<Transparent3d, DrawTrailEffect>();
    }
}

#[derive(Component)]
pub struct TrailEffect {
    pub colour: Color,
    pub duration: f32, // Seconds as f32
    pub start_offset: Vec3,
    pub end_offset: Vec3,
    pub trail_texture: Handle<Image>,
    pub distance_per_point: f32,
}

#[derive(Copy, Clone, Default)]
struct TrailEffectPoint {
    start: Vec3,
    end: Vec3,
    time: f32,
}

#[derive(Component)]
pub struct TrailEffectPositionHistory {
    history: VecDeque<TrailEffectPoint>,
    catmull_points: [TrailEffectPoint; 4],
    trail_length_excess: f32,
    last_temp_points: usize,
}

impl Default for TrailEffectPositionHistory {
    fn default() -> Self {
        Self {
            history: VecDeque::with_capacity(1000),
            catmull_points: Default::default(),
            trail_length_excess: 0.0,
            last_temp_points: 0,
        }
    }
}

pub fn initialise_trail_effects(
    mut commands: Commands,
    query: Query<Entity, (With<TrailEffect>, Without<TrailEffectPositionHistory>)>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(TrailEffectPositionHistory::default());
    }
}

fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t3 = t * t * t;
    let t2 = t * t;

    (-0.5 * t3 + t2 - 0.5 * t) * p0
        + (1.5 * t3 - 2.5 * t2 + 1.0) * p1
        + (-1.5 * t3 + 2.0 * t2 + 0.5 * t) * p2
        + (0.5 * t3 - 0.5 * t2) * p3
}

fn catmull_rom_f32(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t = t;
    let t3 = t * t * t;
    let t2 = t * t;

    (-0.5 * t3 + t2 - 0.5 * t) * p0
        + (1.5 * t3 - 2.5 * t2 + 1.0) * p1
        + (-1.5 * t3 + 2.0 * t2 + 0.5 * t) * p2
        + (0.5 * t3 - 0.5 * t2) * p3
}

pub fn update_trail_effects(
    mut query: Query<(
        &TrailEffect,
        &mut TrailEffectPositionHistory,
        &GlobalTransform,
    )>,
    render_configuration: Res<RenderConfiguration>,
    time: Res<Time>,
) {
    let now = time.elapsed_seconds();

    for (trail_effect, mut history, transform) in query.iter_mut() {
        let transform = transform.compute_transform();
        let point = TrailEffectPoint {
            start: transform.translation
                + transform
                    .rotation
                    .mul_vec3(transform.scale * trail_effect.start_offset),
            end: transform.translation
                + transform
                    .rotation
                    .mul_vec3(transform.scale * trail_effect.end_offset),
            time: now,
        };
        let world_length = point.start.distance(point.end);

        if history.history.is_empty() {
            history.history.push_back(point);
            history.catmull_points.fill(point);
            continue;
        }

        let distance = point.start.distance(history.catmull_points[0].start)
            + point.end.distance(history.catmull_points[0].end);
        let trail_length = history.trail_length_excess + distance;
        let num_points_to_add = (trail_length / trail_effect.distance_per_point) as usize;
        history.trail_length_excess =
            trail_length - (num_points_to_add as f32 * trail_effect.distance_per_point);

        if num_points_to_add > 0 {
            // Shift points
            history.catmull_points.rotate_right(1);
            history.catmull_points[0] = point;

            // Clear last frame's points between 0 and 1
            for _ in 0..history.last_temp_points {
                history.history.pop_front();
            }

            // Between point 1 and 2
            let distance = history.catmull_points[1]
                .start
                .distance(history.catmull_points[2].start)
                + history.catmull_points[1]
                    .end
                    .distance(history.catmull_points[2].end);
            let num_to_add = (distance / trail_effect.distance_per_point) as i32;
            for i in 1..=num_to_add {
                let t = i as f32 / num_to_add as f32;
                let new_start = catmull_rom(
                    history.catmull_points[3].start,
                    history.catmull_points[2].start,
                    history.catmull_points[1].start,
                    history.catmull_points[0].start,
                    t,
                );
                let new_end = catmull_rom(
                    history.catmull_points[3].end,
                    history.catmull_points[2].end,
                    history.catmull_points[1].end,
                    history.catmull_points[0].end,
                    t,
                );
                let new_time = catmull_rom_f32(
                    history.catmull_points[3].time,
                    history.catmull_points[2].time,
                    history.catmull_points[1].time,
                    history.catmull_points[0].time,
                    t,
                );

                history.history.push_front(TrailEffectPoint {
                    start: new_start,
                    end: new_start + (new_end - new_start).normalize() * world_length,
                    time: new_time,
                });

                if history.history.len() == history.history.capacity() {
                    break;
                }
            }

            // Between point 0 and 1
            let distance = history.catmull_points[0]
                .start
                .distance(history.catmull_points[1].start)
                + history.catmull_points[0]
                    .end
                    .distance(history.catmull_points[1].end);
            let num_to_add = (distance / trail_effect.distance_per_point) as i32;
            for i in 1..=num_to_add {
                let t = i as f32 / num_to_add as f32;
                let new_start = catmull_rom(
                    history.catmull_points[2].start,
                    history.catmull_points[1].start,
                    history.catmull_points[0].start,
                    history.catmull_points[0].start,
                    t,
                );
                let new_end = catmull_rom(
                    history.catmull_points[2].end,
                    history.catmull_points[1].end,
                    history.catmull_points[0].end,
                    history.catmull_points[0].end,
                    t,
                );
                let new_time = catmull_rom_f32(
                    history.catmull_points[2].time,
                    history.catmull_points[1].time,
                    history.catmull_points[0].time,
                    history.catmull_points[0].time,
                    t,
                );

                history.history.push_front(TrailEffectPoint {
                    start: new_start,
                    end: new_start + (new_end - new_start).normalize() * world_length,
                    time: new_time,
                });

                if history.history.len() == history.history.capacity() {
                    break;
                }
            }

            // Add new point 0
            history.history.push_front(point);

            // Store the 0->1 num points to be removed next time
            history.last_temp_points = num_to_add as usize + 1;
        }

        // Pop old points
        let last_time =
            now - trail_effect.duration * render_configuration.trail_effect_duration_multiplier;
        while history
            .history
            .back()
            .map_or(false, |point| point.time < last_time)
        {
            history.history.pop_back();
        }
    }
}

#[derive(Resource)]
struct TrailEffectPipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for TrailEffectPipeline {
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
            material_layout,
        }
    }
}

impl SpecializedRenderPipeline for TrailEffectPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: TRAIL_EFFECT_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vs_main".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: 3 * 4 + 2 * 4 + 4,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        // Position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // UV
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 3 * 4,
                            shader_location: 1,
                        },
                        // Color
                        VertexAttribute {
                            format: VertexFormat::Uint32,
                            offset: 3 * 4 + 2 * 4,
                            shader_location: 2,
                        },
                    ],
                }],
                shader_defs: Vec::default(),
            },
            fragment: Some(FragmentState {
                shader: TRAIL_EFFECT_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: match key.contains(MeshPipelineKey::HDR) {
                        true => ViewTarget::TEXTURE_FORMAT_HDR,
                        false => TextureFormat::bevy_default(),
                    },
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.material_layout.clone()],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleStrip,
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
            label: Some("trail_effect_render_pipeline".into()),
            push_constant_ranges: Vec::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TrailEffectVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub colour: u32,
}

struct ExtractedTrailEffect {
    texture: Handle<Image>,
    vertices: Vec<TrailEffectVertex>,
}

#[derive(Default, Resource)]
struct ExtractedTrailEffects {
    trail_effects: Vec<ExtractedTrailEffect>,
}

fn extract_trail_effects(
    mut extracted_trail_effects: ResMut<ExtractedTrailEffects>,
    images: Extract<Res<Assets<Image>>>,
    query: Extract<
        Query<(
            &ComputedVisibility,
            &TrailEffect,
            &TrailEffectPositionHistory,
        )>,
    >,
) {
    let mut trail_index = 0;

    for (visible, trail_effect, position_history) in query.iter() {
        if !visible.is_visible() {
            continue;
        }

        if !images.contains(&trail_effect.trail_texture) {
            continue;
        }

        if position_history.history.len() < 2 {
            continue;
        }

        let required_capacity = 6 * position_history.history.len();
        let mut vertices = if trail_index < extracted_trail_effects.trail_effects.len() {
            // We try to reuse previous allocated vertices
            let mut vertices =
                std::mem::take(&mut extracted_trail_effects.trail_effects[trail_index].vertices);
            vertices.clear();

            if vertices.capacity() < required_capacity {
                vertices.reserve(required_capacity - vertices.capacity());
            }

            vertices
        } else {
            Vec::with_capacity(6 * position_history.history.len())
        };

        let num_points = (position_history.history.len() - 1) as f32;
        let colour = trail_effect.colour.as_rgba_u32();

        for (point_index, point) in position_history.history.iter().enumerate() {
            let uv_x = point_index as f32 / num_points;

            if point_index == 0 {
                // Degenerate
                vertices.push(TrailEffectVertex {
                    position: point.start.to_array(),
                    uv: [uv_x, 0.0],
                    colour,
                });
            }

            vertices.push(TrailEffectVertex {
                position: point.start.to_array(),
                uv: [uv_x, 0.0],
                colour,
            });
            vertices.push(TrailEffectVertex {
                position: point.end.to_array(),
                uv: [uv_x, 1.0],
                colour,
            });

            if point_index + 1 == position_history.history.len() {
                // Degenerate
                vertices.push(TrailEffectVertex {
                    position: point.end.to_array(),
                    uv: [uv_x, 1.0],
                    colour,
                });
            }
        }

        if trail_index < extracted_trail_effects.trail_effects.len() {
            extracted_trail_effects.trail_effects[trail_index] = ExtractedTrailEffect {
                texture: trail_effect.trail_texture.clone_weak(),
                vertices,
            };
        } else {
            extracted_trail_effects
                .trail_effects
                .push(ExtractedTrailEffect {
                    texture: trail_effect.trail_texture.clone_weak(),
                    vertices,
                });
        }

        trail_index += 1;
    }

    extracted_trail_effects.trail_effects.truncate(trail_index);
}

#[derive(Resource)]
struct TrailEffectMeta {
    ranges: Vec<Range<u64>>,
    vertex_count: u64,
    view_bind_group: Option<BindGroup>,
    vertex_buffer: BufferVec<TrailEffectVertex>,
}

impl Default for TrailEffectMeta {
    fn default() -> Self {
        TrailEffectMeta {
            ranges: Vec::default(),
            vertex_count: 0,
            view_bind_group: None,
            vertex_buffer: BufferVec::new(BufferUsages::VERTEX),
        }
    }
}

fn prepare_trail_effects(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut commands: Commands,
    mut trail_effect_meta: ResMut<TrailEffectMeta>,
    mut extracted_trail_effects: ResMut<ExtractedTrailEffects>,
) {
    let mut vertex_count = 0;
    for trail_effect in extracted_trail_effects.trail_effects.iter() {
        vertex_count += trail_effect.vertices.len();
    }

    trail_effect_meta.vertex_count = vertex_count as u64;
    trail_effect_meta.ranges.clear();
    if vertex_count == 0 {
        return;
    }

    trail_effect_meta.vertex_buffer.clear();
    trail_effect_meta
        .vertex_buffer
        .reserve(vertex_count, &render_device);

    extracted_trail_effects
        .trail_effects
        .sort_by(|a, b| a.texture.cmp(&b.texture));

    let mut start: u32 = 0;
    let mut end: u32 = 0;
    let mut current_batch: Option<Handle<Image>> = None;
    for trail_effect in extracted_trail_effects.trail_effects.iter() {
        if start != end {
            if let Some(current_batch_texture) = &current_batch {
                if current_batch_texture != &trail_effect.texture {
                    let current_batch_texture = current_batch.take().unwrap();
                    commands.spawn(TrailEffectBatch {
                        vertex_range: start..end,
                        handle: current_batch_texture,
                    });
                    current_batch = Some(trail_effect.texture.clone_weak());
                    start = end;
                }
            }
        } else {
            current_batch = Some(trail_effect.texture.clone_weak());
        }

        batch_copy(&trail_effect.vertices, &mut trail_effect_meta.vertex_buffer);
        end += trail_effect.vertices.len() as u32;
    }

    if start != end {
        if let Some(current_batch_material) = current_batch {
            commands.spawn(TrailEffectBatch {
                vertex_range: start..end,
                handle: current_batch_material,
            });
        }
    }

    trail_effect_meta
        .vertex_buffer
        .write_buffer(&render_device, &render_queue);
}

fn batch_copy<T: Pod>(src: &[T], dst: &mut BufferVec<T>) {
    for item in src.iter() {
        dst.push(*item);
    }
}

#[derive(Component)]
struct TrailEffectBatch {
    vertex_range: Range<u32>,
    handle: Handle<Image>,
}

#[derive(Default, Resource)]
struct MaterialBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
fn queue_trail_effects(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
    render_device: Res<RenderDevice>,
    mut material_bind_groups: ResMut<MaterialBindGroups>,
    mut trail_effect_meta: ResMut<TrailEffectMeta>,
    view_uniforms: Res<ViewUniforms>,
    trail_effect_pipeline: Res<TrailEffectPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TrailEffectPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    trail_effect_batches: Query<(Entity, &TrailEffectBatch)>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
) {
    if view_uniforms.uniforms.is_empty() || trail_effect_meta.vertex_count == 0 {
        return;
    }

    if let Some(view_bindings) = view_uniforms.uniforms.binding() {
        trail_effect_meta.view_bind_group.get_or_insert_with(|| {
            render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_bindings,
                }],
                label: Some("trail_effect_view_bind_group"),
                layout: &trail_effect_pipeline.view_layout,
            })
        });
    }

    let draw_trail_effect_function = transparent_draw_functions
        .read()
        .get_id::<DrawTrailEffect>()
        .unwrap();

    for (view, mut transparent_phase) in views.iter_mut() {
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        for (entity, batch) in trail_effect_batches.iter() {
            if let Some(gpu_image) = gpu_images.get(&batch.handle) {
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
                        label: Some("trail_effect_material_bind_group"),
                        layout: &trail_effect_pipeline.material_layout,
                    }),
                );
            }

            transparent_phase.add(Transparent3d {
                distance: 10.0, // TODO: Do we need to fix this ?
                pipeline: pipelines.specialize(&pipeline_cache, &trail_effect_pipeline, view_key),
                entity,
                draw_function: draw_trail_effect_function,
            });
        }
    }
}

type DrawTrailEffect = (
    SetItemPipeline,
    SetTrailEffectViewBindGroup<0>,
    SetTrailEffectMaterialBindGroup<1>,
    DrawTrailEffectBatch,
);

struct SetTrailEffectViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTrailEffectViewBindGroup<I> {
    type Param = SRes<TrailEffectMeta>;
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    fn render<'w>(
        _: &P,
        view_uniform: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        trail_effect_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            trail_effect_meta
                .into_inner()
                .view_bind_group
                .as_ref()
                .unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

struct SetTrailEffectMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTrailEffectMaterialBindGroup<I> {
    type Param = SRes<MaterialBindGroups>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<TrailEffectBatch>;

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        batch: ROQueryItem<'w, Self::ItemWorldQuery>,
        material_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
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

struct DrawTrailEffectBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawTrailEffectBatch {
    type Param = SRes<TrailEffectMeta>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<TrailEffectBatch>;

    #[inline]
    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        batch: ROQueryItem<'w, Self::ItemWorldQuery>,
        trail_effect_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_vertex_buffer(
            0,
            trail_effect_meta
                .into_inner()
                .vertex_buffer
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw(batch.vertex_range.clone(), 0..1);
        RenderCommandResult::Success
    }
}
