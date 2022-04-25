#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv0: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uv0: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.uv0 = vertex.uv0;
    return out;
}

[[group(1), binding(0)]]
var water_array_texture: texture_2d_array<f32>;
[[group(1), binding(1)]]
var water_array_sampler: sampler;

struct WaterTextureIndex {
    index: i32;
};
[[group(1), binding(2)]]
var<uniform> water_texture_index: WaterTextureIndex;

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] uv0: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = textureSample(water_array_texture, water_array_sampler, in.uv0, water_texture_index.index);
    output_color = pow(output_color * 2.0, vec4<f32>(2.2)) * lights.ambient_color;
    return output_color;
}
