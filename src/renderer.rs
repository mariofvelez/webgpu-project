use crate::{camera, light, model::{self, Vertex, DrawModel}, scene, texture};
use std::sync::Arc;
use cgmath::SquareMatrix;
use winit::window::Window;
use wgpu::util::DeviceExt;

pub struct Renderer {
	surface: wgpu::Surface<'static>,
	is_surface_configured: bool,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,

	pub texture_bind_group_layouts: [wgpu::BindGroupLayout; 2],

	// uniform buffers
	// vertex
	// TODO: maybe add instance buffer
	camera_model_bind_group: wgpu::BindGroup,
	camera_buffer: wgpu::Buffer,
	model_buffer: wgpu::Buffer, // TODO: change to each model instance containing its own buffer, then bind each one accordingly

	// fragment
	simple_material_bind_group: wgpu::BindGroup,
	simple_material_buffer: wgpu::Buffer,
	light_bind_group: wgpu::BindGroup,
	light_buffer: wgpu::Buffer,

	// rendering
	depth_texture: texture::Texture,
	render_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
	pub async fn new(window: &Arc<Window>) -> anyhow::Result<Self> {
		let size = window.inner_size();

		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			#[cfg(not(target_arch = "wasm32"))]
			backends: wgpu::Backends::PRIMARY,
			#[cfg(target_arch = "wasm32")]
			backends: wgpu::Backends::GL,
			..Default::default()
		});

		let surface = instance.create_surface(window.clone()).unwrap();

		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::default(),
			compatible_surface: Some(&surface),
			force_fallback_adapter: false,
		}).await?;

		let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
			label: None,
			required_features: wgpu::Features::empty(),
			experimental_features: wgpu::ExperimentalFeatures::disabled(),
			required_limits: if cfg!(target_arch = "wasm32") {
				wgpu::Limits::downlevel_webgl2_defaults()
			} else {
				wgpu::Limits::default()
			},
			memory_hints: Default::default(),
			trace: wgpu::Trace::Off,
		}).await?;

		let surface_caps = surface.get_capabilities(&adapter);

		let surface_format = surface_caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_caps.formats[0]);
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};

		// create bind group & layouts for
		// - texture bind group for each material type
		let texture_bind_group_layouts = model::MaterialType::create_texture_bind_group_layouts(&device);
		// - camera, model, and light
		let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Camera Buffer"),
			contents: bytemuck::cast_slice(&[camera::CameraUniform::new()]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let model_uniform: [[f32; 4]; 4] = cgmath::Matrix4::<f32>::identity().into();
		let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Model Buffer"),
			contents: bytemuck::cast_slice(&[model_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let camera_model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
			],
			label: Some("camera_model_bind_group_layout"),
		});
		let camera_model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &camera_model_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: camera_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: model_buffer.as_entire_binding(),
				},
			],
			label: Some("camera_bind_group"),
		});

		// add material uniform
		let simple_material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Simple Material Buffer"),
			contents: bytemuck::cast_slice(&[model::SimpleMaterial::new()]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});
		let simple_material_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				}
			],
			label: Some("simple_material_bind_group_layout"),
		});
		let simple_material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &simple_material_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: model_buffer.as_entire_binding(),
				}
			],
			label: Some("simple_material_bind_group"),
		});

		let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Light Buffer"),
			contents: bytemuck::cast_slice(&[light::LightUniform::new()]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});
		let light_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			}],
			label: None,
		});
		let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &light_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: light_buffer.as_entire_binding(),
			}],
			label: None,
		});

		let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

		// create render pipeline for different material types
		let render_pipeline = {
			let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &[
					&texture_bind_group_layouts[1],
					&camera_model_bind_group_layout,
					&simple_material_bind_group_layout,
					&light_bind_group_layout,
				],
				immediate_size: 0,
			});

			let shader = wgpu::ShaderModuleDescriptor {
				label: Some("Normal Shader"),
				source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
			};

			create_render_pipeline(
				"Normal Render Pipeline",
				&device,
				&layout,
				config.format,
				Some(texture::Texture::DEPTH_FORMAT),
				&[model::ModelVertex::desc()],
				shader,
			)
		};

		Ok(Self {
			surface,
			is_surface_configured: false,
			device,
			queue,
			config,

			texture_bind_group_layouts,

			camera_model_bind_group,
			camera_buffer,
			model_buffer,

			simple_material_bind_group,
			simple_material_buffer,
			light_bind_group,
			light_buffer,

			depth_texture,
			render_pipeline,
		})
	}

	pub fn update_size(&mut self, width: u32, height: u32) {
		self.config.width = width;
		self.config.height = height;
		self.surface.configure(&self.device, &self.config);
		self.is_surface_configured = true;
		self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
	}

	pub fn update_light(&self, light: &light::LightUniform) {
		self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[*light]));
	}

	/*
	Should take in a scene
	*/
	pub fn render(&self, window: &Arc<Window>, camera: &camera::Camera, scene: &scene::Scene) -> Result<(), wgpu::SurfaceError> {
		// update camera buffer
		let camera_uniform = camera::CameraUniform{ view_proj: camera.build_view_projection_matrix().into() };
		self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

		// begin render pass
		window.request_redraw();

		if !self.is_surface_configured {
			return Ok(());
		}

		let output = self.surface.get_current_texture()?;

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: 0.1,
							g: 0.2,
							b: 0.3,
							a: 1.0,
						}),
						store: wgpu::StoreOp::Store,
					},
					depth_slice: None,
				})],
				depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
					view: &self.depth_texture.view,
					depth_ops: Some(wgpu::Operations {
						load: wgpu::LoadOp::Clear(1.0),
						store: wgpu::StoreOp::Store,
					}),
					stencil_ops: None,
				}),
				occlusion_query_set: None,
				timestamp_writes: None,
				multiview_mask: None,
			});

			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_bind_group(1, &self.camera_model_bind_group, &[]);
			render_pass.set_bind_group(2, &self.simple_material_bind_group, &[]);
			render_pass.set_bind_group(3, &self.light_bind_group, &[]);

			// draw scene
			// sort by render pipeline
			// then sort by material type
			// TODO: for now render by same material type, but change later
			self.draw_scene(&mut render_pass, scene);
		}

		// present
		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())
	}

	fn draw_scene<'a>(&self, render_pass: &mut wgpu::RenderPass<'a>, scene: &'a scene::Scene) {
		let models = &scene.models;
		let materials = &scene.materials;
		
		for obj in &scene.objects {
			let transform: [[f32; 4]; 4] = obj.transform.into();
			self.queue.write_buffer(&self.model_buffer, 0, bytemuck::cast_slice(&[transform]));

			let model = &models[obj.model_index];
			for mesh in &model.meshes {
				let material = &materials[mesh.material];
				render_pass.draw_mesh(mesh, material);
			}
		}
	}

	fn create_buffers() {
		// camera buffer

		// model buffer

		// light buffer
	}
}

fn create_render_pipeline(
	label: &str,
	device: &wgpu::Device,
	layout: &wgpu::PipelineLayout,
	color_format: wgpu::TextureFormat,
	depth_format: Option<wgpu::TextureFormat>,
	vertex_layouts: &[wgpu::VertexBufferLayout],
	shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
	let shader = device.create_shader_module(shader);

	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
		label: Some(label),
		layout: Some(layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: Some("vs_main"),
			buffers: vertex_layouts,
			compilation_options: Default::default(),
		},
		fragment: Some(wgpu::FragmentState {
			module: &shader,
			entry_point: Some("fs_main"),
			targets: &[Some(wgpu::ColorTargetState {
				format: color_format,
				blend: Some(wgpu::BlendState {
					alpha: wgpu::BlendComponent::REPLACE,
					color: wgpu::BlendComponent::REPLACE,
				}),
				write_mask: wgpu::ColorWrites::ALL,
			})],
			compilation_options: Default::default(),
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: Some(wgpu::Face::Back),
			polygon_mode: wgpu::PolygonMode::Fill,
			unclipped_depth: false,
			conservative: false,
		},
		depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
			format,
			depth_write_enabled: true,
			depth_compare: wgpu::CompareFunction::Less,
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState {
			count: 1,
			mask: !0,
			alpha_to_coverage_enabled: false,
		},
		multiview_mask: None,
		cache: None,
	})
}