#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
	position: [f32; 3],
	_padding: u32,
	color: [f32; 3],
	_padding2: u32,
}

impl LightUniform {
	pub fn new() -> Self {
		Self {
			position: [2.0, 1.0, 2.0],
			_padding: 0,
			color: [1.0, 1.0, 1.0],
			_padding2: 0,
		}
	}
}