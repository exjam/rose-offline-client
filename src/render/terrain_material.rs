use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, Handle},
    pbr::{
        DrawMesh, DrawPrepass, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, FromWorld, HandleUntyped, Material, MaterialPlugin, Mesh, Plugin, World,
    },
    reflect::TypeUuid,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::SetItemPipeline,
        render_resource::{
            AddressMode, AsBindGroup, AsBindGroupError, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, FilterMode,
            PreparedBindGroup, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor,
            ShaderStages, SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
            VertexFormat,
        },
        renderer::RenderDevice,
        texture::{FallbackImage, Image},
    },
};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};

pub const TERRAIN_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x3d7939250aff89cb);

pub const TERRAIN_MESH_ATTRIBUTE_TILE_INFO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileInfo", 3855645392, VertexFormat::Uint32);

pub const TERRAIN_MATERIAL_MAX_TEXTURES: usize = 100;

#[derive(Default)]
pub struct TerrainMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TERRAIN_MATERIAL_SHADER_HANDLE,
            "shaders/terrain_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(MaterialPlugin::<
            TerrainMaterial,
            DrawTerrainMaterial,
            DrawPrepass<TerrainMaterial>,
        > {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });
    }
}

#[derive(Clone)]
pub struct TerrainMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for TerrainMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        TerrainMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "403e3628-46d2-4d2a-b74c-ce84be2b1ba2"]
pub struct TerrainMaterial {
    pub textures: Vec<Handle<Image>>,
}

impl Material for TerrainMaterial {
    type PipelineData = TerrainMaterialPipelineData;

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
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
            }
        }

        descriptor
            .layout
            .insert(3, pipeline.data.zone_lighting_layout.clone());

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            MESH_ATTRIBUTE_UV_1.at_shader_location(3),
            TERRAIN_MESH_ATTRIBUTE_TILE_INFO.at_shader_location(4),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

impl AsBindGroup for TerrainMaterial {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        image_assets: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        let mut images = vec![];
        for handle in self.textures.iter().take(TERRAIN_MATERIAL_MAX_TEXTURES) {
            match image_assets.get(handle) {
                Some(image) => images.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        let mut textures = vec![&*fallback_image.texture_view; TERRAIN_MATERIAL_MAX_TEXTURES];
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: "terrain_material_bind_group".into(),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&textures[..]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(PreparedBindGroup {
            bindings: vec![],
            bind_group,
            data: (),
        })
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout
    where
        Self: Sized,
    {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "terrain_material_layout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(TERRAIN_MATERIAL_MAX_TEXTURES as u32),
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}

type DrawTerrainMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<TerrainMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawMesh,
);
