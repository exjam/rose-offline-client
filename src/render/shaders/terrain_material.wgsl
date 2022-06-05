#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv0: vec2<f32>;
    [[location(2)]] uv1: vec2<f32>;
    [[location(3)]] tile_info: vec3<i32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv0: vec2<f32>;
    [[location(2)]] uv1: vec2<f32>;
    [[location(3)]] tile_info: vec3<i32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.world_position = world_position;
    out.uv0 = vertex.uv0;
    out.uv1 = vertex.uv1;
    out.tile_info = vertex.tile_info;
    return out;
}

[[group(1), binding(0)]]
var lightmap_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var lightmap_sampler: sampler;
[[group(1), binding(2)]]
var tile_array_texture: texture_2d_array<f32>;
[[group(1), binding(3)]]
var tile_array_sampler: sampler;

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] uv0: vec2<f32>;
    [[location(2)]] uv1: vec2<f32>;
    [[location(3)]] tile_info: vec3<i32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var tile_layer1_id: i32 = in.tile_info.x;
    var tile_layer2_id: i32 = in.tile_info.y;
    var tile_rotation: i32 = in.tile_info.z;
    var layer2_uv: vec2<f32> = in.uv1;
    if (tile_rotation == 2) {
        layer2_uv.x = 1.0 - layer2_uv.x;
    } else if (tile_rotation == 3) {
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 4) {
        layer2_uv.x = 1.0 - layer2_uv.x;
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 5) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = 1.0 - x;
    } else if (tile_rotation == 6) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = x;
    }

    var layer1: vec4<f32> = textureSample(tile_array_texture, tile_array_sampler, in.uv1, tile_layer1_id);
    var layer2: vec4<f32> = textureSample(tile_array_texture, tile_array_sampler, layer2_uv, tile_layer2_id);
    var lightmap: vec4<f32> = textureSample(lightmap_texture, lightmap_sampler, in.uv0);
    var output_color: vec4<f32> = mix(layer1, layer2, layer2.a) * lightmap * 2.0;
    output_color = pow(output_color, vec4<f32>(2.2)) * lights.ambient_color;
    output_color.a = 1.0;
    return output_color;
}
