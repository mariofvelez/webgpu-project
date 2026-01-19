use std::io::{BufReader, Cursor};
use wgpu::util::DeviceExt;
use crate::{model, texture};

#[cfg(target_arch = "wasm32")]
fn format_url(filename: &str) -> reqwest::Url {
	let window = web_sys::window().unwrap();
	let location = window.location();
	let mut origin = location.origin().unwrap();
	if !origin.ends_with("webgpu-project") {
		origin = format!("{}/webgpu-project", origin);
	}
	let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
	base.join(filename).unwrap()
}

pub async fn load_string(filename: &str) -> anyhow::Result<String> {
	#[cfg(target_arch = "wasm32")]
	let txt = {
		let url = format_url(&format!("src/res/{}", filename).as_str());
		reqwest::get(url).await?.text().await?
	};
	#[cfg(not(target_arch = "wasm32"))]
	let txt = {
		let path = std::path::Path::new("src/res").join(filename);
		std::fs::read_to_string(path)?
	};
	Ok(txt)
}

pub async fn load_binary(filename: &str) -> anyhow::Result<Vec<u8>> {
	#[cfg(target_arch = "wasm32")]
	let data = {
		let url = format_url(&format!("src/res/{}", filename).as_str());
		reqwest::get(url).await?.bytes().await?.to_vec()
	};
	#[cfg(not(target_arch = "wasm32"))]
	let data = {
		let path = std::path::Path::new("src/res").join(filename);
		std::fs::read(path)?
	};
	Ok(data)
}

pub async fn load_texture(filename: &str, ty: texture::TextureType, device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<texture::Texture> {
	let data = load_binary(filename).await?;
	texture::Texture::from_bytes(device, queue, &data, filename, ty)
}

struct TobjGeometry<'a> {
	vertices: Vec<model::ModelVertex>,
	indices: &'a Vec<u32>,
}

impl<'a> TobjGeometry<'a> {
	fn from_tobj_mesh(tobj_mesh: &'a tobj::Mesh) -> Self {
		Self {
			vertices: (0..tobj_mesh.positions.len() / 3).map(|i| {
				model::ModelVertex {
				position: [
					tobj_mesh.positions[i * 3],
					tobj_mesh.positions[i * 3 + 1],
					tobj_mesh.positions[i * 3 + 2],
				],
				tex_coords: [
					tobj_mesh.texcoords[i * 2],
					1.0 - tobj_mesh.texcoords[i * 2 + 1],
				],
				normal: [
					tobj_mesh.normals[i * 3],
					tobj_mesh.normals[i * 3 + 1],
					tobj_mesh.normals[i * 3 + 2],
				],
				tangent: [0.0; 4],
			}
			}).collect::<Vec<_>>(),
			indices: &tobj_mesh.indices,
		}
	}
}

impl <'a>mikktspace::Geometry for TobjGeometry<'a> {
	fn num_faces(&self) -> usize {
		self.indices.len() / 3
	}
	fn num_vertices_of_face(&self, _face: usize) -> usize {
		3
	}
	fn position(&self, face: usize, vert: usize) -> [f32; 3] {
		let idx = self.indices[face * 3 + vert] as usize;
		self.vertices[idx].position
	}
	fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
		let idx = self.indices[face * 3 + vert] as usize;
		self.vertices[idx].normal
	}
	fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
		let idx = self.indices[face * 3 + vert] as usize;
		self.vertices[idx].tex_coords
	}
	fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
		let idx = self.indices[face * 3 + vert] as usize;
		self.vertices[idx].tangent = tangent;
	}
}

pub async fn load_model(filename: &str, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> anyhow::Result<model::Model> {
	let obj_text = load_string(filename).await?;
	let obj_cursor = Cursor::new(obj_text);
	let mut obj_reader = BufReader::new(obj_cursor);

	let (models, obj_materials) = tobj::load_obj_buf_async(
		&mut obj_reader,
		&tobj::LoadOptions {
			triangulate: true,
			single_index: true,
			..Default::default()
		},
		|p| async move {
			let mat_text = load_string(&p).await.unwrap();
			tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
		},
	).await?;

	let mut materials = vec![];
	for m in obj_materials? {
		let diffuse_texture = load_texture(&m.diffuse_texture, texture::TextureType::Diffuse, device, queue).await?;
		let normal_texture = load_texture(&m.normal_texture, texture::TextureType::Normal, device, queue).await?;

		materials.push(model::Material::new(
			device, 
			&m.name,
			diffuse_texture,
			normal_texture,
			layout,
		));
	}

	let meshes = models.into_iter().map(|m| {
		// create tobj
		let mut mesh = TobjGeometry::from_tobj_mesh(&m.mesh);

		// create tangents
		mikktspace::generate_tangents(&mut mesh);

		// create vertex & index buffer
		let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some(&format!("{:?} Vertex Buffer", filename)),
			contents: bytemuck::cast_slice(&mesh.vertices),
			usage: wgpu::BufferUsages::VERTEX,
		});
		let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some(&format!("{:?} Index Buffer", filename)),
			contents: bytemuck::cast_slice(&mesh.indices),
			usage: wgpu::BufferUsages::INDEX,
		});

		model::Mesh {
			name: filename.to_string(),
			vertex_buffer,
			index_buffer,
			num_elements: mesh.indices.len() as u32,
			material: m.mesh.material_id.unwrap_or(0),
		}
	}).collect::<Vec<_>>();

	Ok(model::Model {meshes, materials})
}
