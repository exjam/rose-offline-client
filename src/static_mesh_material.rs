use crate::{
    load_internal_asset,
    material::{AlphaMode, DrawMaterial, MaterialPipelineKey, SpecializedMaterial},
    mesh_pipeline::{MeshPipeline, MeshPipelineKey, MeshUniform},
};
use bevy::{
    asset::{AssetServer, Handle},
    core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::Vec2,
    prelude::{
        error, AddAsset, App, FromWorld, HandleUntyped, Mesh, Msaa, Plugin, Query, Res, ResMut,
        World,
    },
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, RenderPipelineCache,
            RenderPipelineDescriptor, SamplerBindingType, ShaderStages, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines, TextureSampleType,
            TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::Image,
        view::{ExtractedView, VisibleEntities},
        RenderApp, RenderStage,
    },
};

pub const STATIC_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 6942039651767701046);

#[derive(Default)]
pub struct StaticMeshMaterialPlugin;

impl Plugin for StaticMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            STATIC_MESH_MATERIAL_SHADER_HANDLE,
            "shaders/static_mesh_material.wgsl",
            Shader::from_wgsl
        );

        app.add_asset::<StaticMeshMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<StaticMeshMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<StaticMeshMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<AlphaMask3d, DrawMaterial<StaticMeshMaterial>>()
                .add_render_command::<Opaque3d, DrawMaterial<StaticMeshMaterial>>()
                .add_render_command::<Transparent3d, DrawMaterial<StaticMeshMaterial>>()
                .init_resource::<StaticMeshMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<StaticMeshMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_static_mesh_material_meshes);
        }
    }
}

pub struct StaticMeshMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
}

impl SpecializedMeshPipeline for StaticMeshMaterialPipeline {
    type Key = MaterialPipelineKey<StaticMeshMaterialKey>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        StaticMeshMaterial::specialize(&mut descriptor, key.material_key, layout)?;
        Ok(descriptor)
    }
}

impl FromWorld for StaticMeshMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = StaticMeshMaterial::bind_group_layout(render_device);

        StaticMeshMaterialPipeline {
            mesh_pipeline: world.get_resource::<MeshPipeline>().unwrap().clone(),
            material_layout,
            vertex_shader: StaticMeshMaterial::vertex_shader(asset_server),
            fragment_shader: StaticMeshMaterial::fragment_shader(asset_server),
        }
    }
}

#[derive(Clone, AsStd140)]
pub struct StaticMeshMaterialLightmapUniformData {
    pub uv_offset: Vec2,
    pub uv_scale: f32,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6942088b-c082-457b-aacf-517228cc0c22"]
pub struct StaticMeshMaterial {
    pub base_texture: Handle<Image>,
    pub alpha_mode: AlphaMode,

    // lightmap texture, uv offset, uv scale
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl Default for StaticMeshMaterial {
    fn default() -> Self {
        Self {
            base_texture: Default::default(),
            alpha_mode: AlphaMode::Opaque,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

/// The GPU representation of a [`StaticMeshMaterial`].
#[derive(Debug, Clone)]
pub struct GpuStaticMeshMaterial {
    pub bind_group: BindGroup,
    pub base_texture: Handle<Image>,
    pub alpha_mode: AlphaMode,
    pub has_lightmap: bool,
    pub lightmap_uniform_buffer: Buffer,
}

impl RenderAsset for StaticMeshMaterial {
    type ExtractedAsset = StaticMeshMaterial;
    type PreparedAsset = GpuStaticMeshMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<StaticMeshMaterialPipeline>,
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
            .get_image_texture(gpu_images, Some(&material.base_texture))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let (lightmap_texture_view, lightmap_texture_sampler) = if let Some(result) =
            material_pipeline
                .mesh_pipeline
                .get_image_texture(gpu_images, material.lightmap_texture.as_ref())
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        };

        let value = StaticMeshMaterialLightmapUniformData {
            uv_offset: material.lightmap_uv_offset,
            uv_scale: material.lightmap_uv_scale,
        };
        let value_std140 = value.as_std140();
        let lightmap_uniform_buffer =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("static_mesh_material_uniform_buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: value_std140.as_bytes(),
            });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(base_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(base_texture_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: lightmap_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(lightmap_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(lightmap_texture_sampler),
                },
            ],
            label: Some("static_mesh_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuStaticMeshMaterial {
            bind_group,
            base_texture: material.base_texture,
            alpha_mode: material.alpha_mode,
            lightmap_uniform_buffer,
            has_lightmap: material.lightmap_texture.is_some(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StaticMeshMaterialKey {
    has_lightmap: bool,
}

impl SpecializedMaterial for StaticMeshMaterial {
    type Key = StaticMeshMaterialKey;

    fn key(render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        StaticMeshMaterialKey {
            has_lightmap: render_asset.has_lightmap,
        }
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Key,
        _layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.has_lightmap {
            descriptor
                .fragment
                .as_mut()
                .unwrap()
                .shader_defs
                .push(String::from("HAS_STATIC_MESH_LIGHTMAP"));
        }
        if let Some(label) = &mut descriptor.label {
            *label = format!("static_mesh_{}", *label).into();
        }
        Ok(())
    }

    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(STATIC_MESH_MATERIAL_SHADER_HANDLE.typed())
    }

    #[inline]
    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(
        render_device: &RenderDevice,
    ) -> bevy::render::render_resource::BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                // Lightmap uniform data
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            StaticMeshMaterialLightmapUniformData::std140_size_static() as u64,
                        ),
                    },
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
            ],
            label: Some("static_mesh_material_layout"),
        })
    }

    #[inline]
    fn alpha_mode(render_asset: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        render_asset.alpha_mode
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn queue_static_mesh_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<StaticMeshMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<StaticMeshMaterialPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<StaticMeshMaterial>>,
    material_meshes: Query<(&Handle<StaticMeshMaterial>, &Handle<Mesh>, &MeshUniform)>,
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
        let draw_opaque = opaque_draw_functions
            .read()
            .get_id::<DrawMaterial<StaticMeshMaterial>>()
            .unwrap();
        let draw_alpha_mask = alpha_mask_draw_functions
            .read()
            .get_id::<DrawMaterial<StaticMeshMaterial>>()
            .unwrap();
        let draw_transparent = transparent_draw_functions
            .read()
            .get_id::<DrawMaterial<StaticMeshMaterial>>()
            .unwrap();

        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if let Some(material) = render_materials.get(material_handle) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mut mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | msaa_key;
                        let alpha_mode = StaticMeshMaterial::alpha_mode(material);
                        if let AlphaMode::Blend = alpha_mode {
                            mesh_key |= MeshPipelineKey::TRANSPARENT_MAIN_PASS;
                        }

                        let material_key = StaticMeshMaterial::key(material);

                        let pipeline_id = pipelines.specialize(
                            &mut pipeline_cache,
                            &material_pipeline,
                            MaterialPipelineKey {
                                mesh_key,
                                material_key,
                            },
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
                        match alpha_mode {
                            AlphaMode::Opaque => {
                                opaque_phase.add(Opaque3d {
                                    entity: *visible_entity,
                                    draw_function: draw_opaque,
                                    pipeline: pipeline_id,
                                    // NOTE: Front-to-back ordering for opaque with ascending sort means near should have the
                                    // lowest sort key and getting further away should increase. As we have
                                    // -z in front of the camera, values in view space decrease away from the
                                    // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                                    distance: -mesh_z,
                                });
                            }
                            AlphaMode::Mask(_) => {
                                alpha_mask_phase.add(AlphaMask3d {
                                    entity: *visible_entity,
                                    draw_function: draw_alpha_mask,
                                    pipeline: pipeline_id,
                                    // NOTE: Front-to-back ordering for alpha mask with ascending sort means near should have the
                                    // lowest sort key and getting further away should increase. As we have
                                    // -z in front of the camera, values in view space decrease away from the
                                    // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                                    distance: -mesh_z,
                                });
                            }
                            AlphaMode::Blend => {
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
    }
}
