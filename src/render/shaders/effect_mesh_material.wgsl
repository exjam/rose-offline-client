#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_functions mesh_normal_local_to_world
#import rose_client::zone_lighting apply_zone_lighting

struct EffectMeshMaterialData {
    flags: u32,
    alpha_cutoff: f32,
};

const EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32 = 0x1u;
const EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32   = 0x2u;

const EFECT_MESH_ANIMATION_STATE_FLAGS_POSITION: u32   = 0x1u;
const EFECT_MESH_ANIMATION_STATE_FLAGS_NORMAL: u32     = 0x2u;
const EFECT_MESH_ANIMATION_STATE_FLAGS_UV: u32         = 0x4u;
const EFECT_MESH_ANIMATION_STATE_FLAGS_ALPHA: u32      = 0x8u;

@group(1) @binding(0)
var<uniform> material: EffectMeshMaterialData;
@group(1) @binding(1)
var base_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_sampler: sampler;

#ifdef HAS_ANIMATION_TEXTURE
@group(1) @binding(3)
var animation_texture: texture_2d<f32>;
@group(1) @binding(4)
var animation_sampler: sampler;

struct AnimationState {
    flags: u32,
    current_next_frame: u32,
    next_weight: f32,
    alpha: f32,
};
var<push_constant> animation_state: AnimationState;
#endif

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
#ifdef VERTEX_NORMALS
    @location(2) normal: vec3<f32>,
#endif
    @builtin(vertex_index) vertex_idx: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
#ifdef VERTEX_NORMALS
    @location(2) world_normal: vec3<f32>,
#endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex.uv;

#ifdef HAS_ANIMATION_TEXTURE
    let current_frame_index = animation_state.current_next_frame & 0xffffu;
    let next_frame_index = animation_state.current_next_frame >> 16u;
    let current_frame_0: vec4<f32> = textureLoad(animation_texture, vec2<u32>(current_frame_index, vertex.vertex_idx), 0);
    let next_frame_0: vec4<f32> = textureLoad(animation_texture, vec2<u32>(next_frame_index, vertex.vertex_idx), 0);

    if ((animation_state.flags & EFECT_MESH_ANIMATION_STATE_FLAGS_POSITION) != 0u) { // Has position ?
        out.world_position = mesh.model * vec4<f32>(mix(current_frame_0.xyz, next_frame_0.xyz, animation_state.next_weight), 1.0);
    } else {
        out.world_position = mesh.model * vec4<f32>(vertex.position, 1.0);
    }

    if ((animation_state.flags & (EFECT_MESH_ANIMATION_STATE_FLAGS_NORMAL | EFECT_MESH_ANIMATION_STATE_FLAGS_UV)) != 0u) {
        let num_frames: u32 = animation_state.flags >> 4u;
        let current_frame_1: vec4<f32> = textureLoad(animation_texture, vec2<u32>(current_frame_index + num_frames, vertex.vertex_idx), 0);
        let next_frame_1: vec4<f32> = textureLoad(animation_texture, vec2<u32>(next_frame_index + num_frames, vertex.vertex_idx), 0);

#ifdef VERTEX_NORMALS
        if ((animation_state.flags & EFECT_MESH_ANIMATION_STATE_FLAGS_NORMAL) != 0u) {
            out.world_normal = mesh_normal_local_to_world(mix(current_frame_1.xyz, next_frame_1.xyz, animation_state.next_weight));
        } else {
            out.world_normal = mesh_normal_local_to_world(vertex.normal);
        }
#endif

        if ((animation_state.flags & EFECT_MESH_ANIMATION_STATE_FLAGS_UV) != 0u) {
            out.uv = vec2<f32>(mix(current_frame_0.w, next_frame_0.w, animation_state.next_weight), mix(current_frame_1.w, next_frame_1.w, animation_state.next_weight));
        } else {
            out.uv = vertex.uv;
        }
    }
#else
    out.world_position = mesh.model * vec4<f32>(vertex.position, 1.0);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
#endif
#endif

    out.clip_position = 1.0 * view.view_proj * out.world_position;
    return out;
}

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
#ifdef VERTEX_NORMALS
    @location(2) world_normal: vec3<f32>,
#endif
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);

#ifdef VERTEX_NORMALS // Only apply lighting to mesh which have normals
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), in.world_position);
    output_color = apply_zone_lighting(in.world_position, in.world_normal, output_color, view_z);
#endif

#ifdef HAS_ANIMATION_TEXTURE
    if ((animation_state.flags & EFECT_MESH_ANIMATION_STATE_FLAGS_ALPHA) != 0u) {
        output_color.a = output_color.a * animation_state.alpha;
    } else
#endif
    if ((material.flags & EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        output_color.a = 1.0;
    } else if ((material.flags & EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        if (output_color.a >= material.alpha_cutoff) {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            output_color.a = 1.0;
        } else {
            // NOTE: output_color.a < material.alpha_cutoff should not is not rendered
            // NOTE: This and any other discards mean that early-z testing cannot be done!
            discard;
        }
    }

    return output_color;
}
