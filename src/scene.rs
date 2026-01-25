use crate::{model, light, camera};

pub struct Scene {
	pub materials: Vec<model::Material>,
	pub models: Vec<model::Model>,
	pub objects: Vec<model::ModelInstance>,
	
	pub light: light::LightUniform,
	pub camera: camera::Camera,
}

impl Scene {
	pub fn new(light: light::LightUniform, camera: camera::Camera) -> Self {
		Self {
			materials: vec![],
			models: vec![],
			objects: vec![],
			light,
			camera,
		}
	}

	pub fn add_model(&mut self, model: model::Model) -> usize {
		self.models.push(model);
		self.materials.len() - 1
	}
	
	pub fn add_material(&mut self, material: model::Material) -> usize {
		self.materials.push(material);
		self.materials.len() - 1
	}

	pub fn get_material(&self, name: &str) -> Option<usize> {
		for (idx, material) in self.materials.iter().enumerate() {
			if material.name == name {
				return Some(idx);
			}
		}
		None
	}

	pub fn add_object(&mut self, obj: model::ModelInstance) {
		self.objects.push(obj);
	}
}