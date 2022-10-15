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
        error, AddAsset, App, Assets, Entity, FromWorld, HandleUntyped, Mesh, Msaa, Plugin, Query,
        Res, ResMut, World,
    },
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendComponent, BlendFactor, BlendOperation, BlendState, PipelineCache,
            RenderPipelineDescriptor, SamplerBindingType, ShaderStages, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines, TextureSampleType,
            TextureViewDimension, VertexFormat,
        },
        renderer::RenderDevice,
        texture::Image,
        view::{ExtractedView, VisibleEntities},
        RenderApp, RenderStage,
    },
};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    TextureArray, MESH_ATTRIBUTE_UV_1,
};

pub const TERRAIN_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x3d7939250aff89cb);

pub const TERRAIN_MESH_ATTRIBUTE_TILE_INFO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileInfo", 3855645392, VertexFormat::Sint32x3);

#[derive(Default)]
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            TERRAIN_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/terrain_material.wgsl")),
        );

        app.add_asset::<TerrainMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<TerrainMaterial>>::extract_visible())
            .add_plugin(RenderAssetPlugin::<TerrainMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawTerrainMaterial>()
                .init_resource::<TerrainMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<TerrainMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_terrain_material_meshes);
        }
    }
}

pub struct TerrainMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub zone_lighting_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
}

impl FromWorld for TerrainMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Lightmap Texture
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
                // Lightmap Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Tile Array Texture
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Tile Array Texture Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("terrain_material_layout"),
        });
        TerrainMaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
            vertex_shader: Some(TERRAIN_MATERIAL_SHADER_HANDLE.typed()),
            fragment_shader: Some(TERRAIN_MATERIAL_SHADER_HANDLE.typed()),
        }
    }
}

impl SpecializedMeshPipeline for TerrainMaterialPipeline {
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

        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.zone_lighting_layout.clone(),
        ]);

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            MESH_ATTRIBUTE_UV_1.at_shader_location(3),
            TERRAIN_MESH_ATTRIBUTE_TILE_INFO.at_shader_location(4),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(descriptor)
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "403e3628-46d2-4d2a-b74c-ce84be2b1ba2"]
pub struct TerrainMaterial {
    pub lightmap_texture: Handle<Image>,
    pub tilemap_texture_array: Handle<TextureArray>,
}

#[derive(Debug, Clone)]
pub struct GpuTerrainMaterial {
    pub bind_group: BindGroup,
    pub lightmap_texture: Handle<Image>,
    pub tilemap_texture_array: Handle<TextureArray>,
}

impl RenderAsset for TerrainMaterial {
    type ExtractedAsset = TerrainMaterial;
    type PreparedAsset = GpuTerrainMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<TerrainMaterialPipeline>,
        SRes<RenderAssets<Image>>,
        SRes<RenderAssets<TextureArray>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_images, gpu_texture_arrays): &mut SystemParamItem<
            Self::Param,
        >,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (lightmap_texture_view, lightmap_texture_sampler) =
            if let Some(lightmap_gpu_image) = gpu_images.get(&material.lightmap_texture) {
                (
                    &lightmap_gpu_image.texture_view,
                    &lightmap_gpu_image.sampler,
                )
            } else {
                return Err(PrepareAssetError::RetryNextUpdate(material));
            };

        let (tile_array_texture_view, tile_array_texture_sampler) =
            if let Some(tile_array_gpu_image) =
                gpu_texture_arrays.get(&material.tilemap_texture_array)
            {
                (
                    &tile_array_gpu_image.texture_view,
                    &tile_array_gpu_image.sampler,
                )
            } else {
                return Err(PrepareAssetError::RetryNextUpdate(material));
            };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(lightmap_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(lightmap_texture_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(tile_array_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(tile_array_texture_sampler),
                },
            ],
            label: Some("terrain_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuTerrainMaterial {
            bind_group,
            lightmap_texture: material.lightmap_texture.clone(),
            tilemap_texture_array: material.tilemap_texture_array.clone(),
        })
    }
}

pub struct SetTerrainMaterialBindGroup<const I: usize>(PhantomData<TerrainMaterial>);
impl<const I: usize> EntityRenderCommand for SetTerrainMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainMaterial>>,
        SQuery<Read<Handle<TerrainMaterial>>>,
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

type DrawTerrainMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTerrainMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawMesh,
);

#[allow(clippy::too_many_arguments)]
pub fn queue_terrain_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    material_pipeline: Res<TerrainMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<TerrainMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<TerrainMaterial>>,
    material_meshes: Query<(&Handle<TerrainMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(&ExtractedView, &VisibleEntities, &mut RenderPhase<Opaque3d>)>,
) {
    for (view, visible_entities, mut opaque_phase) in views.iter_mut() {
        let draw_opaque_pbr = opaque_draw_functions
            .read()
            .get_id::<DrawTerrainMaterial>()
            .unwrap();

        let rangefinder = view.rangefinder3d();
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if render_materials.contains_key(material_handle) {
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

                        let distance = rangefinder.distance(&mesh_uniform.transform);
                        opaque_phase.add(Opaque3d {
                            entity: *visible_entity,
                            draw_function: draw_opaque_pbr,
                            pipeline: pipeline_id,
                            distance,
                        });
                    }
                }
            }
        }
    }
}
