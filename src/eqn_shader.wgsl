struct InstanceInput {
  @location(5) model_matrix_0: vec4<f32>,
  @location(6) model_matrix_1: vec4<f32>,
  @location(7) model_matrix_2: vec4<f32>,
  @location(8) model_matrix_3: vec4<f32>,
  @location(9) color: vec4<f32>,
};

struct CameraUniform {
  view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

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
  instance: InstanceInput,
) -> VertexOutput {
  let model_matrix = mat4x4<f32>(
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );
  var out: VertexOutput;
  out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
  out.model_position = model.position;
  out.color = instance.color;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  var a = 0.0;

  if abs(in.model_position.x - in.model_position.y) <= 0.01 {
    a = 1.0;
  }

  return vec4<f32>(in.color.xyz, a);
}
