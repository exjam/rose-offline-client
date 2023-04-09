use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, Handle},
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::{
        DrawMesh, DrawPrepass, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, Commands, FromWorld, HandleUntyped, Image, IntoSystemAppConfig, Material,
        MaterialPlugin, Mesh, Plugin, Res, Resource, Time, World,
    },
    reflect::TypeUuid,
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase, AddressMode, AsBindGroup, AsBindGroupError, BindGroupDescriptor,
            BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
            FilterMode, PreparedBindGroup, PushConstantRange, RenderPipelineDescriptor,
            SamplerBindingType, SamplerDescriptor, ShaderDefVal, ShaderSize, ShaderStages,
            ShaderType, SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
        Extract, ExtractSchedule, RenderApp,
    },
};

use crate::render::zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta};

pub const WATER_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x333959e64b35d5d9);

pub const WATER_MATERIAL_NUM_TEXTURES: usize = 25;

#[derive(Default)]
pub struct WaterMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WATER_MESH_MATERIAL_SHADER_HANDLE,
            "shaders/water_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(MaterialPlugin::<
            WaterMaterial,
            DrawWaterMaterial,
            DrawPrepass<WaterMaterial>,
        > {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system(extract_water_push_constant_data.in_schedule(ExtractSchedule));
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct WaterPushConstantData {
    pub current_index: i32,
    pub next_index: i32,
    pub next_weight: f32,
}

fn extract_water_push_constant_data(mut commands: Commands, time: Extract<Res<Time>>) {
    let time = time.elapsed_seconds_wrapped() * 10.0;
    let current_index = (time as i32) % WATER_MATERIAL_NUM_TEXTURES as i32;
    let next_index = (current_index + 1) % WATER_MATERIAL_NUM_TEXTURES as i32;
    let next_weight = time.fract();

    commands.insert_resource(WaterPushConstantData {
        current_index,
        next_index,
        next_weight,
    });
}

#[derive(Clone)]
pub struct WaterMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for WaterMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        WaterMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "e9e46dcc-94db-4b31-819f-d5ecffc732f0"]
pub struct WaterMaterial {
    pub textures: Vec<Handle<Image>>,
}

impl Material for WaterMaterial {
    type PipelineData = WaterMaterialPipelineData;

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        WATER_MESH_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        WATER_MESH_MATERIAL_SHADER_HANDLE.typed().into()
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
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }

            // Do not apply color fog to additive blended water
            fragment.shader_defs.push(ShaderDefVal::Bool(
                "ZONE_LIGHTING_DISABLE_COLOR_FOG".into(),
                true,
            ));
        }

        descriptor
            .layout
            .insert(3, pipeline.data.zone_lighting_layout.clone());

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..WaterPushConstantData::SHADER_SIZE.get() as u32,
        });

        Ok(())
    }
}

impl AsBindGroup for WaterMaterial {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        image_assets: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        let mut images = vec![];
        for handle in self.textures.iter().take(WATER_MATERIAL_NUM_TEXTURES) {
            match image_assets.get(handle) {
                Some(image) => images.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        let mut textures = vec![&*fallback_image.texture_view; WATER_MATERIAL_NUM_TEXTURES];
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: "water_material_bind_group".into(),
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
            label: "water_material_layout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(WATER_MATERIAL_NUM_TEXTURES as u32),
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

pub struct SetWaterMaterialPushConstants<const OFFSET: u32>;
impl<P: PhaseItem, const OFFSET: u32> RenderCommand<P> for SetWaterMaterialPushConstants<OFFSET> {
    type Param = SRes<WaterPushConstantData>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        water_uniform_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let byte_buffer = [0u8; WaterPushConstantData::SHADER_SIZE.get() as usize];
        let mut buffer = encase::StorageBuffer::new(byte_buffer);
        buffer.write(water_uniform_data.as_ref()).unwrap();
        pass.set_push_constants(ShaderStages::FRAGMENT, 0, buffer.as_ref());
        RenderCommandResult::Success
    }
}

type DrawWaterMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<WaterMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    SetWaterMaterialPushConstants<0>,
    DrawMesh,
);
