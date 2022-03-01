#import mesh_pipeline::mesh_view_bind_group
#import mesh_pipeline::mesh_struct

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv0: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv0: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.world_position = world_position;
    out.uv0 = vertex.uv0;
    return out;
}

[[group(1), binding(0)]]
var water_array_texture: texture_2d_array<f32>;
[[group(1), binding(1)]]
var water_array_sampler: sampler;

struct WaterData {
    texture_index: i32;
};
[[group(3), binding(0)]]
var<uniform> water_data: WaterData;

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv1: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = textureSample(water_array_texture, water_array_sampler, in.uv1, water_data.texture_index);
    output_color.a = 0.5;
    return output_color;
}
