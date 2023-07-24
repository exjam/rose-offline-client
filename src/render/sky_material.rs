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
    prelude::{App, HandleUntyped, Image, Material, MaterialPlugin, Mesh, Plugin},
    reflect::{TypePath, TypeUuid},
    render::{
        extract_resource::ExtractResourcePlugin,
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            AsBindGroup, CompareFunction, PushConstantRange, RenderPipelineDescriptor,
            ShaderStages, SpecializedMeshPipelineError,
        },
    },
};

use crate::resources::{ZoneTime, ZoneTimeState};

pub const SKY_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0xadc5cbbc7a53fe);

#[derive(Default)]
pub struct SkyMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for SkyMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SKY_MATERIAL_SHADER_HANDLE,
            "shaders/sky_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            ExtractResourcePlugin::<ZoneTime>::default(),
            MaterialPlugin::<SkyMaterial, DrawSkyMaterial, DrawPrepass<SkyMaterial>> {
                prepass_enabled: self.prepass_enabled,
                ..Default::default()
            },
        ));
    }
}

#[derive(Debug, Clone, TypeUuid, TypePath, AsBindGroup)]
#[uuid = "971a6c96-4516-4ea0-aeb6-349633e7934e"]
pub struct SkyMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture_day: Option<Handle<Image>>,

    #[texture(2)]
    #[sampler(3)]
    pub texture_night: Option<Handle<Image>>,
}

impl Material for SkyMaterial {
    type PipelineData = ();

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        SKY_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        bevy::prelude::AlphaMode::Opaque
    }

    fn depth_bias(&self) -> f32 {
        9999999999.0
    }

    fn specialize(
        _: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..4,
        });

        Ok(())
    }
}

struct SetZoneTimePushConstant<const OFFSET: u32>;
impl<P: PhaseItem, const OFFSET: u32> RenderCommand<P> for SetZoneTimePushConstant<OFFSET> {
    type Param = SRes<ZoneTime>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        zone_time: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let day_weight = match zone_time.state {
            ZoneTimeState::Morning => zone_time.state_percent_complete,
            ZoneTimeState::Day => 1.0f32,
            ZoneTimeState::Evening => 1.0f32 - zone_time.state_percent_complete,
            ZoneTimeState::Night => 0.0f32,
        };
        pass.set_push_constants(ShaderStages::FRAGMENT, OFFSET, &day_weight.to_le_bytes());
        RenderCommandResult::Success
    }
}

type DrawSkyMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<SkyMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneTimePushConstant<0>,
    DrawMesh,
);
