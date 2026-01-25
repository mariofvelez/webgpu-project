mod texture;
mod camera;
mod model;
mod resources;
mod scene;
mod renderer;
mod light;


use winit::{
	application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use cgmath::prelude::*;
use std::sync::Arc;

struct Instance {
	position: cgmath::Vector3<f32>,
	rotation: cgmath::Quaternion<f32>,
}

impl Instance {
	fn to_raw(&self) -> InstanceRaw {
		InstanceRaw {
			model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
		}
	}
}

const NUM_INSTANCES_PER_ROW: u32 = 10;
const SPACE_BETWEEN: f32 = 1.0;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
	model: [[f32; 4]; 4]
}

impl InstanceRaw {
	fn desc() -> wgpu::VertexBufferLayout<'static> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 5,
					format: wgpu::VertexFormat::Float32x4,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
					shader_location: 6,
					format: wgpu::VertexFormat::Float32x4,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
					shader_location: 7,
					format: wgpu::VertexFormat::Float32x4,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
					shader_location: 8,
					format: wgpu::VertexFormat::Float32x4,
				},
			],
		}
	}
}

pub struct State {
	pub window: Arc<Window>,
	renderer: renderer::Renderer,
	scene: scene::Scene,
	camera_controller: camera::CameraController,
}

impl State {
	pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
		// create renderer
		let renderer = renderer::Renderer::new(&window).await.unwrap();

		let mut scene = scene::Scene::new(
			light::LightUniform::new(),
			camera::Camera {
				eye: (0.0, 1.0, 2.0).into(),
				target: (0.0, 0.0, 0.0).into(),
				up: cgmath::Vector3::unit_y(),
				aspect: window.inner_size().width as f32 / window.inner_size().height as f32,
				fovy: 45.0,
				znear: 0.1,
				zfar: 100.0,
			},
		);

		let camera_controller = camera::CameraController::new(0.05);

		renderer.update_light(&scene.light);

		let obj = resources::load_model("dragon.obj", &renderer, &mut scene).await.unwrap();
		scene.add_object(
			model::ModelInstance {
				model_index: obj,
				transform: cgmath::Matrix4::identity(),
			}
		);

		Ok(Self {
			window,
			renderer,
			scene,
			camera_controller,
		})
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		if width > 0 && height > 0 {
			self.renderer.update_size(width, height);
			self.scene.camera.update_aspect(width, height);
		}
	}

	pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
		if code == KeyCode::Escape && is_pressed {
			event_loop.exit();
		} else {
			self.camera_controller.handle_key(code, is_pressed);
		}
	}

	fn update(&mut self) {
		self.camera_controller.update_camera(&mut self.scene.camera);
	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		self.renderer.render(&self.window, &self.scene.camera, &self.scene)
	}
}

pub struct App {
	#[cfg(target_arch = "wasm32")]
	proxy: Option<winit::event_loop::EventLoopProxy<State>>,
	state: Option<State>,
}

impl App {
	pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
		#[cfg(target_arch = "wasm32")]
		let proxy = Some(event_loop.create_proxy());
		Self {
			state: None,
			#[cfg(target_arch = "wasm32")]
			proxy,
		}
	}
}

impl ApplicationHandler<State> for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		#[allow(unused_mut)]
		let mut window_attributes = Window::default_attributes();

		#[cfg(target_arch = "wasm32")]
		{
			use wasm_bindgen::JsCast;
			use winit::platform::web::WindowAttributesExtWebSys;

			const CANVAS_ID: &str = "canvas";

			let window = wgpu::web_sys::window().unwrap_throw();
			let document = window.document().unwrap_throw();
			let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
			let html_canvas_element = canvas.unchecked_into();
			window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
		}

		let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
		window.set_title("WebGPU yay");

		#[cfg(not(target_arch = "wasm32"))]
		{
			self.state = Some(pollster::block_on(State::new(window)).unwrap());
		}

		#[cfg(target_arch = "wasm32")]
		{
			if let Some(proxy) = self.proxy.take() {
				wasm_bindgen_futures::spawn_local(async move {
					assert!(proxy.send_event(State::new(window).await.expect("Unable to create canvas!")).is_ok())
				});
			}
		}
	}

	#[allow(unused_mut)]
	fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
		#[cfg(target_arch = "wasm32")]
		{
			event.window.request_redraw();
			event.resize(
				event.window.inner_size().width,
				event.window.inner_size().height,
			);
		}
		self.state = Some(event);
	}

	fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
		let state = match &mut self.state {
			Some(canvas) => canvas,
			None => return,
		};

		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => state.resize(size.width, size.height),
			WindowEvent::RedrawRequested => {
				state.update();
				match state.render() {
					Ok(_) => {},
					Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
						let size = state.window.inner_size();
						state.resize(size.width, size.height);
					}
					Err(e) => {
						log::error!("Unable to render {}", e);
					}
				}
			}
			WindowEvent::KeyboardInput {
				event:
					KeyEvent {
						physical_key: PhysicalKey::Code(code),
						state: key_state,
						..
					},
					..
			} => state.handle_key(event_loop, code, key_state.is_pressed()),
			_ => {}
		}
	}
}

pub fn run() -> anyhow::Result<()> {
	#[cfg(not(target_arch = "wasm32"))]
	{
		env_logger::init();
	}
	#[cfg(target_arch = "wasm32")]
	{
		console_log::init_with_level(log::Level::Info).unwrap_throw();
	}

	let event_loop = EventLoop::with_user_event().build()?;
	let mut app = App::new(
		#[cfg(target_arch = "wasm32")]
		&event_loop,
	);
	event_loop.run_app(&mut app)?;

	Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
	console_error_panic_hook::set_once();
	run().unwrap_throw();

	Ok(())
}