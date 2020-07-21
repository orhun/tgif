pub mod settings;

use crate::edit::settings::EditSettings;
use crate::image::geometry::Geometry;
use image::imageops::{self, FilterType};
use image::{ImageBuffer, RgbaImage};
use std::convert::TryInto;

/* Image editor */
#[derive(Clone, Debug)]
pub struct Editor {
	pub image: RgbaImage,
	pub geometry: Geometry,
	settings: EditSettings,
}

impl Editor {
	/**
	 * Create a new Editor object.
	 *
	 * @param  settings
	 * @return Editor
	 */
	pub fn new(settings: EditSettings) -> Self {
		Self {
			image: ImageBuffer::new(0, 0),
			geometry: Geometry::default(),
			settings,
		}
	}

	/**
	 * Set the geometry to use while editing.
	 *
	 * @param size
	 */
	pub fn init(&mut self, size: (u32, u32)) {
		let (mut width, mut height) = if !self.settings.resize.is_zero() {
			(self.settings.resize.width, self.settings.resize.height)
		} else {
			size
		};
		if self.settings.ratio > 0. && self.settings.ratio != 1. {
			let (w, h) = (width, height);
			width = (w as f32 * self.settings.ratio) as u32;
			height = (h as f32 * self.settings.ratio) as u32;
		}
		self.geometry =
			Geometry::new(0, 0, width, height).with_padding(self.settings.crop);
	}

	/**
	 * Edit and return the image.
	 *
	 * @param  image
	 * @return RgbaImage
	 */
	pub fn edit(&mut self, image: RgbaImage) -> RgbaImage {
		self.image = image;
		self.resize().crop().image.clone()
	}

	/* Resize the image */
	fn resize(&mut self) -> &mut Self {
		if !self.settings.resize.is_zero()
			|| (self.settings.ratio > 0. && self.settings.ratio != 1.)
		{
			self.image = imageops::resize(
				&self.image,
				self.geometry.width,
				self.geometry.height,
				FilterType::Lanczos3,
			);
		}
		self
	}

	/* Crop the image */
	fn crop(&mut self) -> &mut Self {
		if !self.settings.crop.is_zero() {
			self.image = imageops::crop(
				&mut self.image,
				self.geometry.x.try_into().unwrap_or_default(),
				self.geometry.y.try_into().unwrap_or_default(),
				self.geometry.width,
				self.geometry.height,
			)
			.to_image();
		}
		self
	}
}