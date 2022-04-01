#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex.uv;
    out.world_position = mesh.model * vec4<f32>(vertex.position, 1.0);
    out.clip_position = view.view_proj * out.world_position;
    return out;
}

struct EffectMeshMaterialData {
    flags: u32;
    alpha_cutoff: f32;
};

let EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32              = 1u;
let EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32                = 2u;

[[group(1), binding(0)]]
var<uniform> material: EffectMeshMaterialData;
[[group(1), binding(1)]]
var base_texture: texture_2d<f32>;
[[group(1), binding(2)]]
var base_sampler: sampler;

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);

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
