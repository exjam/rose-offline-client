use std::{cmp::Ordering, ops::Range};

use bevy::{
    asset::{Handle, HandleId},
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::MeshPipelineKey,
    prelude::{
        App, Assets, Color, Commands, Component, ComputedVisibility, Entity, FromWorld,
        GlobalTransform, HandleUntyped, Msaa, Plugin, Query, Res, ResMut, Resource, Vec2, Vec3,
        World,
    },
    reflect::TypeUuid,
    render::{
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferSize,
            BufferUsages, BufferVec, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, FragmentState, FrontFace, MultisampleState,
            PipelineCache, PolygonMode, PrimitiveState, PrimitiveTopology,
            RenderPipelineDescriptor, SamplerBindingType, ShaderStages, SpecializedRenderPipeline,
            SpecializedRenderPipelines, StencilFaceState, StencilState, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexAttribute, VertexBufferLayout,
            VertexFormat, VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{ExtractedView, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract, RenderApp, RenderStage,
    },
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

use crate::render::zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta};

pub const WORLD_UI_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xd5cdda11c713e3a7);

#[derive(Default)]
pub struct WorldUiRenderPlugin;

impl Plugin for WorldUiRenderPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            WORLD_UI_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/world_ui.wgsl")),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_system_to_stage(RenderStage::Extract, extract_world_ui_rects)
                .init_resource::<ExtractedWorldUi>()
                .init_resource::<WorldUiMeta>()
                .init_resource::<ImageBindGroups>()
                .add_render_command::<Transparent3d, DrawWorldUi>()
                .init_resource::<WorldUiPipeline>()
                .init_resource::<SpecializedRenderPipelines<WorldUiPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_world_ui_meshes);
        }
    }
}

#[derive(Component, Clone)]
pub struct WorldUiRect {
    pub image: Handle<Image>,
    pub screen_offset: Vec2,
    pub screen_size: Vec2,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub color: Color,
    pub order: u8,
}

pub struct ExtractedRect {
    pub world_position: Vec3,
    pub screen_offset: Vec2,
    pub screen_size: Vec2,
    pub image_handle_id: HandleId,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub color: Color,
    pub order: u8,
}

#[derive(Resource)]
pub struct ExtractedWorldUi {
    pub rects: Vec<ExtractedRect>,
}

impl Default for ExtractedWorldUi {
    fn default() -> Self {
        Self {
            rects: Vec::with_capacity(1024),
        }
    }
}

fn extract_world_ui_rects(
    mut extracted_world_ui: ResMut<ExtractedWorldUi>,
    images: Extract<Res<Assets<Image>>>,
    query: Extract<Query<(&ComputedVisibility, &GlobalTransform, &WorldUiRect)>>,
) {
    extracted_world_ui.rects.clear();
    for (visible, global_transform, rect) in query.iter() {
        if !visible.is_visible_in_hierarchy() {
            continue;
        }

        if !images.contains(&rect.image) {
            continue;
        }

        extracted_world_ui.rects.push(ExtractedRect {
            world_position: global_transform.translation(),
            screen_offset: rect.screen_offset,
            screen_size: rect.screen_size,
            image_handle_id: rect.image.id(),
            uv_min: rect.uv_min,
            uv_max: rect.uv_max,
            color: rect.color,
            order: rect.order,
        });
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct WorldUiVertex {
    world_position: [f32; 3],
    screen_position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[derive(Resource)]
pub struct WorldUiMeta {
    vertices: BufferVec<WorldUiVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for WorldUiMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct WorldUiPipeline {
    view_layout: BindGroupLayout,
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    material_layout: BindGroupLayout,
    zone_lighting_layout: BindGroupLayout,
}

impl SpecializedRenderPipeline for WorldUiPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                entry_point: "vertex".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: 3 * 4 + 2 * 4 + 2 * 4 + 4 * 4,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        // World Position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // Screen Position
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 3 * 4,
                            shader_location: 1,
                        },
                        // UV
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 3 * 4 + 2 * 4,
                            shader_location: 2,
                        },
                        // Color
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 3 * 4 + 2 * 4 + 2 * 4,
                            shader_location: 3,
                        },
                    ],
                }],
                shader_defs: vec!["ZONE_LIGHTING_GROUP_2".into()],
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: vec!["ZONE_LIGHTING_GROUP_2".into()],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
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
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: Some(vec![
                self.view_layout.clone(),
                self.material_layout.clone(),
                self.zone_lighting_layout.clone(),
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
            label: Some("world_ui_pipeline".into()),
        }
    }
}

impl FromWorld for WorldUiPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

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
            entries: &[
                // Base Texture
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
                // Base Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("world_ui_material_layout"),
        });

        WorldUiPipeline {
            view_layout,
            vertex_shader: WORLD_UI_SHADER_HANDLE.typed(),
            fragment_shader: WORLD_UI_SHADER_HANDLE.typed(),
            material_layout,
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

pub struct SetWorldUiMaterialBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetWorldUiMaterialBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SQuery<Read<WorldUiBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_batch = query_batch.get(item).unwrap();
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(
            I,
            image_bind_groups
                .values
                .get(&Handle::weak(sprite_batch.image_handle_id))
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

type DrawWorldUi = (
    SetItemPipeline,
    SetWorldUiViewBindGroup<0>,
    SetWorldUiMaterialBindGroup<1>,
    SetZoneLightingBindGroup<2>,
    DrawWorldUiBatch,
);

struct DrawWorldUiBatch;
impl EntityRenderCommand for DrawWorldUiBatch {
    type Param = (SRes<WorldUiMeta>, SQuery<Read<WorldUiBatch>>);

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (sprite_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batch = query_batch.get(item).unwrap();
        let sprite_meta = sprite_meta.into_inner();
        pass.set_vertex_buffer(0, sprite_meta.vertices.buffer().unwrap().slice(..));
        pass.draw(batch.vertex_range.clone(), 0..1);
        RenderCommandResult::Success
    }
}

struct SetWorldUiViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetWorldUiViewBindGroup<I> {
    type Param = (SRes<WorldUiMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (world_ui_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            world_ui_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

#[derive(Component, Eq, PartialEq, Clone)]
pub struct WorldUiBatch {
    image_handle_id: HandleId,
    vertex_range: Range<u32>,
}

#[derive(Default, Resource)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_world_ui_meshes(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    world_ui_pipeline: Res<WorldUiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<WorldUiPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
    mut extracted_world_ui: ResMut<ExtractedWorldUi>,
    mut world_ui_meta: ResMut<WorldUiMeta>,
    mut commands: Commands,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    view_uniforms: Res<ViewUniforms>,
) {
    if view_uniforms.uniforms.is_empty() {
        return;
    }

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);
    let pipeline = pipelines.specialize(&mut pipeline_cache, &world_ui_pipeline, msaa_key);
    let draw_alpha_mask = transparent_draw_functions
        .read()
        .get_id::<DrawWorldUi>()
        .unwrap();

    if let Some(view_bindings) = view_uniforms.uniforms.binding() {
        world_ui_meta.view_bind_group.get_or_insert_with(|| {
            render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_bindings,
                }],
                label: Some("world_ui_view_bind_group"),
                layout: &world_ui_pipeline.view_layout,
            })
        });
    }

    for (view, mut transparent_phase) in views.iter_mut() {
        let inverse_view_transform = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_transform.row(2);
        let view_proj = view.projection * inverse_view_transform;
        let view_width = view.viewport.z as f32;
        let view_height = view.viewport.w as f32;

        extracted_world_ui.rects.sort_unstable_by(|a, b| {
            match view_proj
                .project_point3(a.world_position)
                .z
                .partial_cmp(&view_proj.project_point3(b.world_position).z)
            {
                Some(Ordering::Equal) | None => a.order.cmp(&b.order),
                Some(other) => other,
            }
        });

        world_ui_meta.vertices.clear();
        world_ui_meta
            .vertices
            .reserve(extracted_world_ui.rects.len() * 6, &render_device);

        for rect in extracted_world_ui.rects.iter() {
            let gpu_image =
                if let Some(gpu_image) = gpu_images.get(&Handle::weak(rect.image_handle_id)) {
                    gpu_image
                } else {
                    // Image not ready yet, ignore
                    continue;
                };

            let clip_pos = view_proj.project_point3(rect.world_position);
            if clip_pos.z < 0.0 || clip_pos.z > 1.0 {
                // Outside frustum depth, ignore
                continue;
            }
            let screen_pos =
                (clip_pos.truncate() + Vec2::ONE) / 2.0 * Vec2::new(view_width, view_height);

            let min_screen_pos = screen_pos + rect.screen_offset;
            let max_screen_pos = screen_pos + rect.screen_offset + rect.screen_size;
            if max_screen_pos.x < 0.0
                || max_screen_pos.y < 0.0
                || min_screen_pos.x >= view_width
                || min_screen_pos.y >= view_height
            {
                // Not visible on screen
                continue;
            }

            let positions = [
                [rect.screen_offset.x, rect.screen_offset.y],
                [
                    rect.screen_offset.x + rect.screen_size.x,
                    rect.screen_offset.y,
                ],
                [
                    rect.screen_offset.x + rect.screen_size.x,
                    rect.screen_offset.y + rect.screen_size.y,
                ],
                [
                    rect.screen_offset.x,
                    rect.screen_offset.y + rect.screen_size.y,
                ],
            ];
            let uvs = [
                [rect.uv_min.x, rect.uv_max.y],
                [rect.uv_max.x, rect.uv_max.y],
                [rect.uv_max.x, rect.uv_min.y],
                [rect.uv_min.x, rect.uv_min.y],
            ];

            const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];
            let color = rect.color.as_linear_rgba_f32();

            let item_start = world_ui_meta.vertices.len() as u32;
            for i in QUAD_INDICES {
                world_ui_meta.vertices.push(WorldUiVertex {
                    world_position: rect.world_position.to_array(),
                    screen_position: positions[i],
                    uv: uvs[i],
                    color,
                });
            }
            let item_end = world_ui_meta.vertices.len() as u32;

            let visible_entity = commands
                .spawn(WorldUiBatch {
                    image_handle_id: rect.image_handle_id,
                    vertex_range: item_start..item_end,
                })
                .id();

            image_bind_groups
                .values
                .entry(Handle::weak(rect.image_handle_id))
                .or_insert_with(|| {
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
                        label: Some("world_ui_bind_group"),
                        layout: &world_ui_pipeline.material_layout,
                    })
                });

            transparent_phase.add(Transparent3d {
                entity: visible_entity,
                draw_function: draw_alpha_mask,
                pipeline,
                distance: inverse_view_row_2.dot(rect.world_position.extend(1.0)),
            });
        }
    }

    world_ui_meta
        .vertices
        .write_buffer(&render_device, &render_queue);
}
