@group(2) @binding(0)
var<uniform> camera: mat4x4<f32>;

@group(2) @binding(1)
var<uniform> model: mat4x4<f32>;

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

// struct InstanceInput {
// 	@location(5) model_matrix_0: vec4<f32>,
// 	@location(6) model_matrix_1: vec4<f32>,
// 	@location(7) model_matrix_2: vec4<f32>,
// 	@location(8) model_matrix_3: vec4<f32>,
// };

@vertex
fn vs_main(
	vertex_input: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;
	var world_pos = model * vec4<f32>(vertex_input.position, 1.0);
	out.position = world_pos.xyz;
	out.tex_coords = vertex_input.tex_coords;
	out.normal = (model * vec4<f32>(vertex_input.normal, 0.0)).xyz;
	var tangent = model * vec4<f32>(vertex_input.tangent.xyz, 0.0);
	out.tangent = vec4<f32>(tangent.xyz, vertex_input.tangent.w);
	out.clip_position = camera * world_pos;
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

@group(1) @binding(0)
var cubemap_texture: texture_cube<f32>;
@group(1) @binding(1)
var cubemap_sampler: sampler;

struct SimpleMaterial {
	diffuse_spec: vec4<f32>,
	roughness: f32,
	metal: f32,
};
@group(2) @binding(2)
var<uniform> material: SimpleMaterial;

struct Light {
	position: vec3<f32>,
	color: vec3<f32>,
};
@group(2) @binding(3)
var<uniform> light: Light;

@group(2) @binding(4)
var<uniform> camera_pos: vec4<f32>;

fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let obj_col = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
	let tangent_norm = textureSample(normal_texture, normal_sampler, in.tex_coords).xyz * 2.0 - 1.0; // normal in tangent space

	let bitangent = cross(in.normal, in.tangent.xyz) * in.tangent.w;
	let obj_norm = normalize(tangent_norm.x * in.tangent.xyz + tangent_norm.y * bitangent + tangent_norm.z * in.normal);
	let light_dir = normalize(light.position - in.position);
	let eye_dir = normalize(camera_pos.xyz - in.position);

	let reflect_strength = fresnel_schlick(max(dot(eye_dir, obj_norm), 0.0), material.diffuse_spec.w);
	let cubemap_col = textureSample(cubemap_texture, cubemap_sampler, reflect(-eye_dir, obj_norm)).xyz * reflect_strength;

	let ambient_strength = 0.1;
	let ambient_col = light.color * ambient_strength;

	let diffuse_strength = max(dot(obj_norm, light_dir), 0.0) * (1.0 - reflect_strength);
	let diffuse_col = light.color * diffuse_strength;

	let result = (diffuse_col + cubemap_col) * obj_col.xyz;
	return vec4<f32>(result, obj_col.w);
}