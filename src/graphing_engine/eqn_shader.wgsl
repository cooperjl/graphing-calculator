struct CameraUniform {
  view_proj: mat4x4<f32>,
};

struct ColorUniform {
  raw: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> color: ColorUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) model_position: vec3<f32>,
  @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
  model: VertexInput,
) -> VertexOutput {
  var out: VertexOutput;
  out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
  out.color = color.raw;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(in.color);
}
