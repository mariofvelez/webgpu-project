use std::ops::Range;
use cgmath;

use crate::texture;

pub trait Vertex {
	fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
	pub position: [f32; 3],
	pub tex_coords: [f32; 2],
	pub normal: [f32; 3],
	pub tangent: [f32; 4],
}

impl Vertex for ModelVertex {
	fn desc() -> wgpu::VertexBufferLayout<'static> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute { // position
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute { // tex coords
					offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x2,
				},
				wgpu::VertexAttribute { // normal
					offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute { // tangent
					offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
					shader_location: 3,
					format: wgpu::VertexFormat::Float32x4,
				},
			],
		}
	}
}

pub struct Model {
	pub meshes: Vec<Mesh>,
}

pub struct ModelInstance {
	pub model_index: usize,
	pub transform: cgmath::Matrix4::<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
	pub transform: [[f32; 4]; 4],
}

pub enum MaterialType {
	SingleColorMaterial([f32; 3]),
	DiffuseMapMaterial(texture::Texture),
	DiffuseNormalMapMaterial(texture::Texture, texture::Texture),
	//PbrMaterial(texture::Texture, texture::Texture, texture::Texture),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimpleMaterial {
	pub diffuse_spec: [f32; 4],
	pub roughness: f32,
	pub metal: f32,
}

impl SimpleMaterial {
	pub fn new() -> Self {
		Self {
			diffuse_spec: [1.0, 0.0, 0.0, 0.5],
			roughness: 0.5,
			metal: 0.0,
		}
	}
}

impl MaterialType {
	pub fn create_texture_bind_group_layouts(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {

		let diffuse_texture_entry = wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::FRAGMENT,
			ty: wgpu::BindingType::Texture {
				multisampled: false,
				view_dimension: wgpu::TextureViewDimension::D2,
				sample_type: wgpu::TextureSampleType::Float {filterable: true},
			},
			count: None,
		};
		let diffuse_sampler_entry = wgpu::BindGroupLayoutEntry {
			binding: 1,
			visibility: wgpu::ShaderStages::FRAGMENT,
			ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
			count: None,
		};
		let normal_texture_entry = wgpu::BindGroupLayoutEntry {
			binding: 2,
			visibility: wgpu::ShaderStages::FRAGMENT,
			ty: wgpu::BindingType::Texture {
				multisampled: false,
				view_dimension: wgpu::TextureViewDimension::D2,
				sample_type: wgpu::TextureSampleType::Float {filterable: true},
			},
			count: None,
		};
		let normal_sampler_entry = wgpu::BindGroupLayoutEntry {
			binding: 3,
			visibility: wgpu::ShaderStages::FRAGMENT,
			ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
			count: None,
		};

		[
			device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				entries: &[diffuse_texture_entry.clone(), diffuse_sampler_entry.clone()],
				label: Some("DiffuseMap texture_bind_group_layout"),
			}),
			device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				entries: &[
					diffuse_texture_entry.clone(), 
					diffuse_sampler_entry.clone(),
					normal_texture_entry.clone(),
					normal_sampler_entry.clone(),
				],
				label: Some("DiffuseNormalMap texture_bind_group_layout"),
			}),
		]
	}
}

pub struct Material {
	pub name: String,
	pub diffuse_texture: texture::Texture,
	pub normal_texture: texture::Texture,
	pub bind_group: wgpu::BindGroup,
}

impl Material {
	pub fn new(
		device: &wgpu::Device,
		name: &str,
		diffuse_texture: texture::Texture,
		normal_texture: texture::Texture,
		layout: &wgpu::BindGroupLayout,
	) -> Self {
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(&normal_texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 3,
					resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
				},
			],
			label: Some(name),
		});

		Self {
			name: String::from(name),
			diffuse_texture,
			normal_texture,
			bind_group,
		}
	}
}

pub struct Mesh {
	pub name: String,
	pub vertex_buffer: wgpu::Buffer,
	pub index_buffer: wgpu::Buffer,
	pub num_elements: u32,
	pub material: usize,
}

pub trait DrawModel<'a> {
	fn draw_mesh(
		&mut self,
		mesh: &'a Mesh,
		material: &'a Material,
	);
	fn draw_mesh_instanced(
		&mut self,
		mesh: &'a Mesh,
		material: &'a Material,
		instances: Range<u32>
	);
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a> where 'b: 'a, {
	fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material) {
		self.draw_mesh_instanced(mesh, material, 0..1);
	}
	fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, material: &'b Material, instances: Range<u32>) {
		self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
		self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
		self.set_bind_group(0, &material.bind_group, &[]);
		self.draw_indexed(0..mesh.num_elements, 0, instances);
	}
}