use image::GenericImageView;
use anyhow::*;

pub enum TextureType {
	Diffuse,
	Normal,
	Cubemap,
}

pub struct Texture {
	#[allow(unused)]
	pub texture: wgpu::Texture,
	pub view: wgpu::TextureView,
	pub sampler: wgpu::Sampler,
}

impl Texture {
	pub fn from_bytes(
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		bytes: &[u8],
		label: &str,
		ty: TextureType,
	) -> Result<Self> {
		let img = image::load_from_memory(bytes)?;
		Self::from_images(device, queue, &vec![img], Some(label), ty)
	}

	pub fn from_images(
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		imgs: &Vec<image::DynamicImage>,
		label: Option<&str>,
		ty: TextureType,
	) -> Result<Self> {
		let dimensions = imgs[0].dimensions();
		println!("dimensions: {:?}", dimensions);

		let texture_size = wgpu::Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: match ty {
				TextureType::Cubemap => 6,
				_ => 1,
			},
		};
		let texture = device.create_texture(
			&wgpu::TextureDescriptor {
				label,
				size: texture_size,
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format: match ty {
					TextureType::Normal => wgpu::TextureFormat::Rgba8Unorm,
					_ => wgpu::TextureFormat::Rgba8UnormSrgb,
				},
				usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
				view_formats: &[],
			},
		);

		for (idx, img) in imgs.iter().enumerate() {
			let rgba = img.to_rgba8();
			queue.write_texture(
				wgpu::TexelCopyTextureInfo {
					texture: &texture,
					mip_level: 0,
					origin: wgpu::Origin3d {
						x: 0,
						y: 0,
						z: idx as u32,
					},
					aspect: wgpu::TextureAspect::All,
				},
				&rgba,
				wgpu::TexelCopyBufferLayout {
					offset: 0,
					bytes_per_row: Some(4 * dimensions.0),
					rows_per_image: Some(dimensions.1),
				},
				wgpu::Extent3d {
					width: dimensions.0,
					height: dimensions.1,
					depth_or_array_layers: 1,
				},
			);
		}

		let view = texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Texture View"),
			dimension: match ty {
				TextureType::Cubemap => Some(wgpu::TextureViewDimension::Cube),
				_ => Some(wgpu::TextureViewDimension::default())
			},
			..Default::default()
		});
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::MipmapFilterMode::Nearest,
			..Default::default()
		});

		Ok(Self{ texture, view, sampler })
	}

	pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

	pub fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> Self {
		let size = wgpu::Extent3d {
			width: config.width.max(1),
			height: config.height.max(1),
			depth_or_array_layers: 1,
		};
		let desc = wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: Self::DEPTH_FORMAT,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		};
		let texture = device.create_texture(&desc);

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::MipmapFilterMode::Nearest,
			compare: Some(wgpu::CompareFunction::LessEqual),
			lod_min_clamp: 0.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self {texture, view, sampler}
	}
}