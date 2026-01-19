struct CameraUniform {
	view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
	position: vec3<f32>,
	color: vec3<f32>,
};
@group(2) @binding(0)
var<uniform> light: Light;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) tex_coords: vec2<f32>,
	@location(2) normal: vec3<f32>,
	@location(3) tangent: vec4<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) position: vec3<f32>,
	@location(1) tex_coords: vec2<f32>,
	@location(2) normal: vec3<f32>,
	@location(3) tangent: vec4<f32>,
};

struct InstanceInput {
	@location(5) model_matrix_0: vec4<f32>,
	@location(6) model_matrix_1: vec4<f32>,
	@location(7) model_matrix_2: vec4<f32>,
	@location(8) model_matrix_3: vec4<f32>,
};

@vertex
fn vs_main(
	model: VertexInput,
	instance: InstanceInput,
) -> VertexOutput {
	let model_matrix = mat3x3<f32> (
		instance.model_matrix_0.xyz,
		instance.model_matrix_1.xyz,
		instance.model_matrix_2.xyz,
	);
	var out: VertexOutput;
	var world_pos = model_matrix * vec3<f32>(model.position) + instance.model_matrix_3.xyz;
	out.position = world_pos.xyz;
	out.tex_coords = model.tex_coords;
	out.normal = model_matrix * model.normal;
	out.tangent = vec4<f32>(model_matrix * model.tangent.xyz, model.tangent.w);
	out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
	return out;
}

@group(0) @binding(0)
var diffuse_texture: texture_2d<f32>;
@group(0) @binding(1)
var diffuse_sampler: sampler;
@group(0) @binding(2)
var normal_texture: texture_2d<f32>;
@group(0) @binding(3)
var normal_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let obj_col = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
	let tangent_norm = textureSample(normal_texture, normal_sampler, in.tex_coords).xyz * 2.0 - 1.0; // normal in tangent space

	let bitangent = cross(in.normal, in.tangent.xyz) * in.tangent.w;
	let obj_norm = normalize(tangent_norm.x * in.tangent.xyz + tangent_norm.y * bitangent + tangent_norm.z * in.normal);

	let ambient_strength = 0.1;
	let ambient_col = light.color * ambient_strength;

	let light_dir = normalize(light.position - in.position);
	let diffuse_strength = max(dot(obj_norm, light_dir), 0.0) * 0.9;
	let diffuse_col = light.color * diffuse_strength;

	let result = diffuse_col * obj_col.xyz;
	return vec4<f32>(result, obj_col.a);
}