use crate::{
    load_internal_asset,
    material::{DrawMaterial, MaterialPipelineKey, SpecializedMaterial},
    mesh_pipeline::{MeshPipeline, MeshPipelineKey, MeshUniform},
};
use bevy::{
    asset::{AssetServer, Handle},
    core_pipeline::Opaque3d,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
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
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            FilterMode, RenderPipelineCache, RenderPipelineDescriptor, SamplerBindingType,
            SamplerDescriptor, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::Image,
        view::{ExtractedView, VisibleEntities},
        RenderApp, RenderStage,
    },
};

pub const TERRAIN_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4206939651767701046);

#[derive(Default)]
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TERRAIN_MATERIAL_SHADER_HANDLE,
            "shaders/terrain_material.wgsl",
            Shader::from_wgsl
        );

        app.add_asset::<TerrainMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<TerrainMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<TerrainMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawMaterial<TerrainMaterial>>()
                .init_resource::<TerrainMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<TerrainMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_terrain_material_meshes);
        }
    }
}

pub struct TerrainMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
}

impl SpecializedMeshPipeline for TerrainMaterialPipeline {
    type Key = MaterialPipelineKey<TerrainMaterialKey>;

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

        TerrainMaterial::specialize(&mut descriptor, key.material_key, layout)?;
        Ok(descriptor)
    }
}

impl FromWorld for TerrainMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = TerrainMaterial::bind_group_layout(render_device);

        TerrainMaterialPipeline {
            mesh_pipeline: world.get_resource::<MeshPipeline>().unwrap().clone(),
            material_layout,
            vertex_shader: TerrainMaterial::vertex_shader(asset_server),
            fragment_shader: TerrainMaterial::fragment_shader(asset_server),
        }
    }
}

/// A material with "standard" properties used in PBR lighting
/// Standard property values with pictures here
/// <https://google.github.io/filament/Material%20Properties.pdf>.
///
/// May be created directly from a [`Color`] or an [`Image`].
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7494888b-c082-457b-aacf-517228cc0c22"]
pub struct TerrainMaterial {
    pub lightmap_texture: Handle<Image>,
    pub tile_array_texture: Handle<Image>,
}

/// The GPU representation of a [`TerrainMaterial`].
#[derive(Debug, Clone)]
pub struct GpuTerrainMaterial {
    pub bind_group: BindGroup,
    pub lightmap_texture: Handle<Image>,
    pub tile_array_texture: Handle<Image>,
}

impl RenderAsset for TerrainMaterial {
    type ExtractedAsset = TerrainMaterial;
    type PreparedAsset = GpuTerrainMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<TerrainMaterialPipeline>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, pbr_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (lightmap_texture_view, _) =
            if let Some(gpu_image) = gpu_images.get(&material.lightmap_texture) {
                (&gpu_image.texture_view, &gpu_image.sampler)
            } else {
                return Err(PrepareAssetError::RetryNextUpdate(material));
            };
        let (tile_array_texture_view, _) =
            if let Some(gpu_image) = gpu_images.get(&material.tile_array_texture) {
                (&gpu_image.texture_view, &gpu_image.sampler)
            } else {
                return Err(PrepareAssetError::RetryNextUpdate(material));
            };

        let sampler_descriptor = SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        };
        let lightmap_sampler = render_device.create_sampler(&sampler_descriptor);
        let tile_array_sampler = render_device.create_sampler(&sampler_descriptor);

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(lightmap_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&lightmap_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(tile_array_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&tile_array_sampler),
                },
            ],
            label: Some("pbr_standard_material_bind_group"),
            layout: &pbr_pipeline.material_layout,
        });

        Ok(GpuTerrainMaterial {
            bind_group,
            lightmap_texture: material.lightmap_texture,
            tile_array_texture: material.tile_array_texture,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TerrainMaterialKey;

impl SpecializedMaterial for TerrainMaterial {
    type Key = TerrainMaterialKey;

    fn key(_render_asset: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        TerrainMaterialKey {}
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::Key,
        _layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(label) = &mut descriptor.label {
            *label = format!("pbr_{}", *label).into();
        }
        Ok(())
    }

    fn fragment_shader(_asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(TERRAIN_MATERIAL_SHADER_HANDLE.typed())
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
            label: Some("pbr_material_layout"),
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_terrain_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    material_pipeline: Res<TerrainMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<TerrainMaterialPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<TerrainMaterial>>,
    material_meshes: Query<(&Handle<TerrainMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(&ExtractedView, &VisibleEntities, &mut RenderPhase<Opaque3d>)>,
) {
    for (view, visible_entities, mut opaque_phase) in views.iter_mut() {
        let draw_opaque_pbr = opaque_draw_functions
            .read()
            .get_id::<DrawMaterial<TerrainMaterial>>()
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
                        let mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | msaa_key;
                        let material_key = TerrainMaterial::key(material);

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
                        opaque_phase.add(Opaque3d {
                            entity: *visible_entity,
                            draw_function: draw_opaque_pbr,
                            pipeline: pipeline_id,
                            // NOTE: Front-to-back ordering for opaque with ascending sort means near should have the
                            // lowest sort key and getting further away should increase. As we have
                            // -z in front of the camera, values in view space decrease away from the
                            // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                            distance: -mesh_z,
                        });
                    }
                }
            }
        }
    }
}
