#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings
#import rose_client::zone_lighting

@group(2) @binding(0)
var<uniform> mesh: Mesh;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) uv0: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv0: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.world_position = world_position;
    out.uv0 = vertex.uv0;
    return out;
}

@group(1) @binding(0)
var water_array_texture: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var water_array_sampler: sampler;

struct WaterTextureIndex {
    current_index: i32,
    next_index: i32,
    next_weight: f32,
};
var<push_constant> water_texture_index: WaterTextureIndex;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv0: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), in.world_position);

    let color1 = textureSample(water_array_texture[water_texture_index.current_index], water_array_sampler, in.uv0);
    let color2 = textureSample(water_array_texture[water_texture_index.next_index], water_array_sampler, in.uv0);
    let water_color = mix(color1, color2, water_texture_index.next_weight);
    let lit_color = apply_zone_lighting(in.world_position, water_color, view_z);
    let srgb_color = pow(lit_color, vec4<f32>(2.2));
    return vec4<f32>(srgb_color.rgb, 1.0);
}
