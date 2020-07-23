pub mod settings;

use crate::edit::settings::{EditSettings, Flip};
use crate::image::geometry::Geometry;
use image::imageops::{self, colorops, FilterType};
use image::{DynamicImage, ImageBuffer, RgbaImage};
use std::convert::TryInto;

/* Image processor */
#[derive(Clone, Debug)]
pub struct ImageOps<'a> {
	pub image: RgbaImage,
	pub geometry: Geometry,
	settings: EditSettings<'a>,
}

impl<'a> ImageOps<'a> {
	/**
	 * Create a new ImageOps object.
	 *
	 * @param  settings
	 * @return ImageOps
	 */
	pub fn new(settings: EditSettings<'a>) -> Self {
		Self {
			image: ImageBuffer::new(0, 0),
			geometry: Geometry::default(),
			settings,
		}
	}

	/**
	 * Set the geometry to use while processing.
	 *
	 * @param size
	 */
	pub fn init(&mut self, size: (u32, u32)) {
		let (mut width, mut height) = if !self.settings.resize.is_zero() {
			(self.settings.resize.width, self.settings.resize.height)
		} else {
			size
		};
		if self.settings.ratio > 0.
			&& (self.settings.ratio - 1.).abs() > f32::EPSILON
		{
			let (w, h) = (width, height);
			width = (w as f32 * self.settings.ratio) as u32;
			height = (h as f32 * self.settings.ratio) as u32;
		}
		if self.settings.rotate == 90 || self.settings.rotate == 270 {
			let (w, h) = (width, height);
			width = h;
			height = w;
		}
		self.geometry =
			Geometry::new(0, 0, width, height).with_padding(self.settings.crop);
	}

	/**
	 * Process and return the image.
	 *
	 * @param  image
	 * @return RgbaImage
	 */
	pub fn process(&mut self, image: RgbaImage) -> RgbaImage {
		self.image = image;
		self.crop()
			.flip()
			.rotate()
			.resize()
			.blur()
			.update_colors()
			.image
			.clone()
	}

	/* Resize the image */
	fn resize(&mut self) -> &mut Self {
		if !self.settings.resize.is_zero()
			|| (self.settings.ratio > 0.
				&& (self.settings.ratio - 1.).abs() > f32::EPSILON)
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

	/* Flip the image */
	fn flip(&mut self) -> &mut Self {
		match self.settings.flip {
			Some(Flip::Horizontal) => {
				imageops::flip_horizontal_in_place(&mut self.image)
			}
			Some(Flip::Vertical) => {
				imageops::flip_vertical_in_place(&mut self.image)
			}
			_ => {}
		}
		self
	}

	/* Rotate the image */
	fn rotate(&mut self) -> &mut Self {
		if self.settings.rotate == 90 {
			self.image = imageops::rotate90(&self.image);
		} else if self.settings.rotate == 180 {
			self.image = imageops::rotate180(&self.image);
		} else if self.settings.rotate == 270 {
			self.image = imageops::rotate270(&self.image);
		}
		self
	}

	/* Blur the image */
	fn blur(&mut self) -> &mut Self {
		if self.settings.blur > 0. {
			self.image = imageops::blur(&self.image, self.settings.blur);
		}
		self
	}

	/* Update the colors of the image */
	fn update_colors(&mut self) -> &mut Self {
		if self.settings.color.grayscale {
			self.image =
				DynamicImage::ImageLuma8(colorops::grayscale(&self.image)).to_rgba();
		}
		if self.settings.color.invert {
			colorops::invert(&mut self.image);
		}
		if self.settings.color.brightness != 0 {
			self.image =
				colorops::brighten(&self.image, self.settings.color.brightness);
		}
		if self.settings.color.hue != 0 {
			self.image = colorops::huerotate(&self.image, self.settings.color.hue);
		}
		if self.settings.color.contrast != 0. {
			self.image =
				colorops::contrast(&self.image, self.settings.color.contrast);
		}
		self
	}
}
