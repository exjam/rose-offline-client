#import bevy_render::view View
#import rose_client::zone_lighting apply_zone_lighting_fog

@group(0) @binding(0)
var<uniform> view: View;

struct Vertex {
    @location(0) world_position: vec3<f32>,
    @location(1) screen_offset: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    out.world_position = vec4<f32>(vertex.world_position, 1.0);

    // Transform the world position to clip space
    out.clip_position = view.view_proj * out.world_position * 1.0f;

    // Clip to normalized device coordinate space
    out.clip_position = out.clip_position / out.clip_position.w;

    // Offset by the proportion of the screen in x and y.
    out.clip_position.x = out.clip_position.x + (vertex.screen_offset.x * 2.0 / view.viewport.z);
    out.clip_position.y = out.clip_position.y + (vertex.screen_offset.y * 2.0 / view.viewport.w);

    out.uv = vertex.uv;
    out.color = vertex.color;
    return out;
}

@group(1) @binding(0)
var base_texture: texture_2d<f32>;
@group(1) @binding(1)
var base_sampler: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), in.world_position);
    let texture_color = textureSample(base_texture, base_sampler, in.uv) * in.color;
    return apply_zone_lighting_fog(in.world_position, texture_color, view_z);
}
