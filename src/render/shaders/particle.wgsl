struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;

struct PositionBuffer { data: array<vec4<f32>>, };
struct SizeBuffer { data: array<vec2<f32>>, };
struct ColorBuffer { data: array<vec4<f32>>, };
struct TextureBuffer { data: array<vec4<f32>>, };

@group(1) @binding(0)
var<storage, read> positions: PositionBuffer;
@group(1) @binding(1)
var<storage, read> sizes: SizeBuffer;
@group(1) @binding(2)
var<storage, read> colors: ColorBuffer;
@group(1) @binding(3)
var<storage, read> textures: TextureBuffer;
@group(2) @binding(0)
var base_color_texture: texture_2d<f32>;
@group(2) @binding(1)
var base_color_sampler: sampler;

struct VertexInput {
  @builtin(vertex_index) vertex_idx: u32,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
  var vertex_positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0),
  );

  let vert_idx = model.vertex_idx % 6u;
  let particle_idx = model.vertex_idx / 6u;

#ifdef PARTICLE_BILLBOARD_Y_AXIS
  let camera_right =
    normalize(vec3<f32>(view.view_proj.x.x, 0.0, view.view_proj.z.x));
  let camera_up = vec3<f32>(0.0, 1.0, 0.0);
#else

#ifdef PARTICLE_BILLBOARD_FULL
  let camera_right =
    normalize(vec3<f32>(view.view_proj.x.x, view.view_proj.y.x, view.view_proj.z.x));
  let camera_up =
    normalize(vec3<f32>(view.view_proj.x.y, view.view_proj.y.y, view.view_proj.z.y));
#else

  let camera_right = vec3<f32>(1.0, 0.0, 0.0);
  let camera_up = vec3<f32>(0.0, 0.0, 1.0);

#endif

#endif

  let particle_position = positions.data[particle_idx].xyz;
  let theta = positions.data[particle_idx].w;
  let size = sizes.data[particle_idx];
  let sin_cos = vec2<f32>(cos(theta), sin(theta));

  let rotation = mat2x2<f32>(
    vec2<f32>(sin_cos.x, -sin_cos.y),
    vec2<f32>(sin_cos.y, sin_cos.x),
  );

  let vertex_position = rotation * vertex_positions[vert_idx];

  var world_space: vec3<f32> =
    particle_position +
    (camera_right * vertex_position.x * size.x) +
    (camera_up * vertex_position.y * size.y);

  var out: VertexOutput;
  out.position = view.view_proj * vec4<f32>(world_space, 1.0);
  out.color = colors.data[particle_idx];

  let texture = textures.data[particle_idx];
  if (vertex_positions[vert_idx].x < 0.0) {
    out.uv.x = texture.x;
  } else {
    out.uv.x = texture.z;
  }

  if (vertex_positions[vert_idx].y > 0.0) {
    out.uv.y = texture.y;
  } else {
    out.uv.y = texture.w;
  }

  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.color * pow(textureSample(base_color_texture, base_color_sampler, in.uv), vec4<f32>(2.2));
}
