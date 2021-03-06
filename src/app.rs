use crate::anim::decoder::AnimDecoder;
use crate::anim::Frames;
use crate::apng::ApngEncoder;
use crate::args::Args;
use crate::file::format::FileFormat;
use crate::file::File as FileUtil;
use crate::gif::encoder::{Encoder, EncoderConfig};
#[cfg(feature = "ski")]
use crate::gif::ski::GifskiEncoder;
use crate::gif::GifEncoder;
use crate::image::Image;
use crate::record::Recorder;
use crate::settings::AppSettings;
use crate::window::Capture;
use bytesize::ByteSize;
use image::bmp::BmpEncoder;
use image::codecs::png::PngDecoder;
use image::farbfeld::FarbfeldEncoder;
use image::gif::GifDecoder;
use image::ico::IcoEncoder;
use image::io::Reader;
use image::jpeg::JpegEncoder;
use image::png::PngEncoder;
use image::pnm::{PnmEncoder, PnmSubtype};
use image::tga::TgaEncoder;
use image::tiff::TiffEncoder;
use image::{
	AnimationDecoder, ColorType, ExtendedColorType, ImageEncoder, ImageFormat,
};
use std::convert::TryInto;
use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{self, Error, Read, Write};
use std::path::Path;
use std::thread;

/* Application output and result types */
pub type AppOutput = (Option<Image>, Option<Frames>);
pub type AppResult = Result<(), Error>;

/* Application and main functionalities */
#[derive(Clone, Copy, Debug)]
pub struct App<'a, Window> {
	window: Option<Window>,
	settings: &'a AppSettings<'a>,
}

impl<'a, Window> App<'a, Window>
where
	Window: Capture + Send + Sync + Copy + Debug + 'static,
{
	/**
	 * Create a new App object.
	 *
	 * @param  window (Option)
	 * @param  settings
	 * @return App
	 */
	pub fn new(window: Option<Window>, settings: &'a AppSettings<'a>) -> Self {
		Self { window, settings }
	}

	/**
	 * Start the application.
	 *
	 * @return Result
	 */
	pub fn start(&self) -> AppResult {
		trace!("Window: {:?}", self.window);
		debug!("{:?}", self.settings.save.file);
		debug!("Command: {:?}", self.settings.record.get_command());
		if let Some(misc_args) = self.settings.args.subcommand_matches("misc") {
			if let Some(shell) = misc_args.value_of("gen-completions") {
				Args::gen_completions(shell, &mut io::stdout());
			}
		} else if self.settings.args.is_present("split") {
			info!("Reading frames from {:?}...", self.settings.split.file);
			self.split_anim(File::open(&self.settings.split.file)?)?;
			info!(
				"Frames saved to {:?} in {} format.",
				self.settings.split.dir,
				self.settings.save.file.format.as_extension().to_uppercase(),
			);
		} else if self.settings.args.is_present("analyze") {
			debug!("Analyzing the image... ({:?})", self.settings.analyze.file);
			self.analyze_image()?;
		} else if self.settings.save.file.path.to_str() == Some("-") {
			self.save_output(self.get_app_output(), io::stdout());
		} else {
			self.save_output(
				self.get_app_output(),
				File::create(&self.settings.save.file.path)?,
			);
			info!(
				"{} saved to: {:?} ({})",
				self.settings.save.file.format.as_extension().to_uppercase(),
				self.settings.save.file.path,
				ByteSize(fs::metadata(&self.settings.save.file.path)?.len())
			);
		}
		Ok(())
	}

	/**
	 * Get the application output.
	 *
	 * @return AppOutput
	 */
	fn get_app_output(self) -> AppOutput {
		let output = if self.settings.save.file.format.is_animation() {
			(None, Some(self.get_frames()))
		} else {
			(self.get_image(), None)
		};
		if let Some(window) = self.window {
			window.release();
		}
		output
	}

	/**
	 * Get the image to save.
	 *
	 * @return Image (Option)
	 */
	fn get_image(self) -> Option<Image> {
		if self.settings.args.is_present("edit") {
			debug!("{:?}", self.settings.edit);
			info!("Opening {:?}...", self.settings.edit.path);
			Some(self.edit_image(&self.settings.edit.path))
		} else {
			self.capture()
		}
	}

	/**
	 * Get the frames to save.
	 *
	 * @return Frames
	 */
	fn get_frames(self) -> Frames {
		if self.settings.args.is_present("edit") {
			info!("Reading frames from {:?}...", self.settings.edit.path);
			self.edit_anim(
				File::open(&self.settings.edit.path).expect("File not found"),
				&self.settings.edit.path,
			)
		} else if self.settings.args.is_present("make") {
			info!(
				"Making an animation from {} frames...",
				self.settings.anim.frames.len()
			);
			let mut images = Vec::new();
			for path in &self.settings.anim.frames {
				debug!("Reading a frame from {:?}   \r", path);
				io::stdout().flush().expect("Failed to flush stdout");
				images.push(self.edit_image(path));
			}
			debug!("\n");
			(images, self.settings.anim.fps)
		} else {
			(self.record(), self.settings.anim.fps)
		}
	}

	/**
	 * Capture the image of window.
	 *
	 * @return Image (Option)
	 */
	fn capture(self) -> Option<Image> {
		let window = self.window.expect("Failed to get the window");
		if self.settings.record.command.is_some() {
			let image_thread = thread::spawn(move || {
				window.show_countdown();
				info!("Capturing an image...");
				window.get_image()
			});
			self.settings
				.record
				.get_command()
				.expect("No command specified to run")
				.execute()
				.expect("Failed to run the command");
			image_thread
				.join()
				.expect("Failed to join the image thread")
		} else {
			window.show_countdown();
			info!("Capturing an image...");
			window.get_image()
		}
	}

	/**
	 * Start recording the frames.
	 *
	 * @return Vector of Image
	 */
	fn record(self) -> Vec<Image> {
		let mut recorder = Recorder::new(
			self.window.expect("Failed to get the window"),
			self.settings.anim.fps,
			self.settings.anim.gifski.0,
			self.settings.record,
		);
		if self.settings.record.command.is_some() {
			let record = recorder.record_async();
			self.settings
				.record
				.get_command()
				.expect("No command specified to run")
				.execute()
				.expect("Failed to run the command");
			match record.get() {
				Some(frames) => frames.expect("Failed to retrieve the frames"),
				None => Vec::new(),
			}
		} else {
			recorder.record_sync(if self.settings.record.flag.keys.is_some() {
				self.settings.input_state
			} else {
				None
			})
		}
	}

	/**
	 * Edit and return the image.
	 *
	 * @param  path
	 * @return Image
	 */
	fn edit_image(self, path: &Path) -> Image {
		let image = Reader::open(path)
			.expect("File not found")
			.with_guessed_format()
			.expect("File format not supported")
			.decode()
			.expect("Failed to decode the image")
			.to_rgba8();
		self.settings
			.edit
			.get_imageops()
			.init(image.dimensions())
			.process(image)
			.get_image()
	}

	/**
	 * Analyze the image and return/save the report.
	 *
	 * @return Result
	 */
	fn analyze_image(self) -> AppResult {
		let analyzer = self.settings.analyze.get_analyzer();
		if self.settings.save.file.format == FileFormat::Txt {
			fs::write(&self.settings.save.file.path, analyzer.get_report() + "\n")?;
			info!(
				"Report saved to: {:?} ({})",
				self.settings.save.file.path,
				ByteSize(fs::metadata(&self.settings.save.file.path)?.len())
			);
		} else {
			info!("{}#", analyzer.get_colored_report());
		}
		Ok(())
	}

	/**
	 * Return the updated frames after decoding the animation.
	 *
	 * @param  input
	 * @param  path
	 * @return Frames
	 */
	fn edit_anim<Input: Read>(self, input: Input, path: &Path) -> Frames {
		let format = Reader::open(path)
			.expect("File not found")
			.with_guessed_format()
			.expect("File format not supported")
			.format();
		AnimDecoder::new(self.settings.edit.get_imageops(), &self.settings.anim)
			.update_frames(
				match format {
					Some(ImageFormat::Gif) => GifDecoder::new(input)
						.expect("Failed to create GIF decoder")
						.into_frames()
						.collect_frames()
						.ok(),
					Some(ImageFormat::Png) => PngDecoder::new(input)
						.expect("Failed to create PNG decoder")
						.apng()
						.into_frames()
						.collect_frames()
						.ok(),
					_ => None,
				}
				.expect("Failed to collect animation frames"),
			)
	}

	/**
	 * Split animation into frames.
	 *
	 * @param  input
	 * @return Frames
	 */
	fn split_anim<Input: Read>(self, input: Input) -> AppResult {
		let (frames, fps) = self.edit_anim(input, &self.settings.split.file);
		debug!("FPS: {}", fps);
		fs::create_dir_all(&self.settings.split.dir)?;
		for i in 0..frames.len() {
			let path = FileUtil::get_path_with_extension(
				self.settings.split.dir.join(format!("frame_{}", i,)),
				&self.settings.save.file.format,
			);
			debug!("Saving to {:?}\r", path);
			io::stdout().flush().expect("Failed to flush stdout");
			self.save_output((frames.get(i).cloned(), None), File::create(path)?);
		}
		debug!("\n");
		Ok(())
	}

	/**
	 * Save the application output.
	 *
	 * @param  app_output
	 * @param  output
	 */
	fn save_output<Output: Write>(&self, app_output: AppOutput, mut output: Output) {
		let (image, frames) = app_output;
		match self.settings.save.file.format {
			FileFormat::Gif => {
				debug!("{:?}", self.settings.anim);
				self.save_gif(frames, output);
			}
			FileFormat::Apng => {
				debug!("{:?}", self.settings.anim);
				self.save_apng(frames, output);
			}
			FileFormat::Png => self.save_image(
				image,
				PngEncoder::new_with_quality(
					output,
					self.settings.png.compression,
					self.settings.png.filter,
				),
				ExtendedColorType::Rgba8,
			),
			FileFormat::Jpg => self.save_image(
				image,
				JpegEncoder::new_with_quality(
					&mut output,
					self.settings.jpg.quality,
				),
				ExtendedColorType::Rgb8,
			),
			FileFormat::Bmp => self.save_image(
				image,
				BmpEncoder::new(&mut output),
				ExtendedColorType::Rgba8,
			),
			FileFormat::Ico => self.save_image(
				image,
				IcoEncoder::new(output),
				ExtendedColorType::Rgba8,
			),
			FileFormat::Tiff => self.save_image(
				image,
				TiffEncoder::new(
					File::create(&self.settings.save.file.path)
						.expect("Failed to create file for TIFF"),
				),
				ExtendedColorType::Rgba8,
			),
			FileFormat::Tga => self.save_image(
				image,
				TgaEncoder::new(output),
				ExtendedColorType::Rgba8,
			),
			FileFormat::Pnm(_) => self.save_image(
				image,
				PnmEncoder::new(output).with_subtype(self.settings.pnm.subtype),
				match self.settings.pnm.subtype {
					PnmSubtype::Bitmap(_) => ExtendedColorType::L1,
					PnmSubtype::Graymap(_) => ExtendedColorType::L8,
					PnmSubtype::Pixmap(_) => ExtendedColorType::Rgb8,
					PnmSubtype::ArbitraryMap => ExtendedColorType::Rgba8,
				},
			),
			FileFormat::Ff => self.save_image(
				image,
				FarbfeldEncoder::new(output),
				ExtendedColorType::Rgba16,
			),
			_ => {}
		}
	}

	/**
	 * Save the image to a file.
	 *
	 * @param image (Option)
	 * @param encoder
	 * @param color_type
	 */
	fn save_image<Encoder: ImageEncoder>(
		self,
		image: Option<Image>,
		encoder: Encoder,
		color_type: ExtendedColorType,
	) {
		let image = image.expect("Failed to get the image");
		if !self.settings.args.is_present("split") {
			info!(
				"Saving the image as {}...",
				self.settings.save.file.format.as_extension().to_uppercase()
			);
			debug!("{:?}", image);
			debug!("{:?}", self.settings.png);
			debug!("{:?}", self.settings.jpg);
			debug!("{:?}", self.settings.pnm);
			debug!("Color type: {:?}", color_type);
		}
		encoder
			.write_image(
				&image.get_data(color_type),
				image.geometry.width,
				image.geometry.height,
				match color_type {
					ExtendedColorType::L1 | ExtendedColorType::L8 => ColorType::L8,
					ExtendedColorType::Rgb8 => ColorType::Rgb8,
					ExtendedColorType::Rgba16 => ColorType::Rgba16,
					_ => ColorType::Rgba8,
				},
			)
			.expect("Failed to encode the image");
	}

	/**
	 * Save frames to a GIF file.
	 *
	 * @param  frames (Option)
	 * @param  output
	 */
	#[cfg(feature = "ski")]
	fn save_gif<Output: Write>(self, frames: Option<Frames>, output: Output) {
		let (images, fps) = frames.expect("Failed to get the frames");
		let config = EncoderConfig::new(
			fps,
			images.first().expect("No frames found to save").geometry,
			output,
			&self.settings.anim,
		);
		if self.settings.anim.gifski.0 {
			GifskiEncoder::new(config).save(images, self.settings.input_state)
		} else {
			GifEncoder::new(config).save(images, self.settings.input_state)
		}
	}

	/**
	 * Save frames to a GIF file.
	 *
	 * @param  frames (Option)
	 * @param  output
	 */
	#[cfg(not(feature = "ski"))]
	fn save_gif<Output: Write>(self, frames: Option<Frames>, output: Output) {
		let (images, fps) = frames.expect("Failed to get the frames");
		GifEncoder::new(EncoderConfig::new(
			fps,
			images.first().expect("No frames found to save").geometry,
			output,
			&self.settings.anim,
		))
		.save(images, self.settings.input_state)
	}

	/**
	 * Save frames to a APNG file.
	 *
	 * @param  frames (Option)
	 * @param  output
	 */
	fn save_apng<Output: Write>(self, frames: Option<Frames>, output: Output) {
		let images = frames.expect("Failed to get the frames").0;
		ApngEncoder::new(
			images.len().try_into().unwrap_or_default(),
			images.first().expect("No frames found to save").geometry,
			&self.settings.anim,
		)
		.save(images, self.settings.input_state, output);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::args::matches::ArgMatches;
	use crate::window::test::TestWindow;
	use clap::ArgMatches as Args;
	use std::env;
	use std::path::PathBuf;
	#[test]
	fn test_app_image() -> AppResult {
		let args = Args::default();
		let matches = ArgMatches::new(&args);
		let mut settings = AppSettings::new(&matches);
		let window = TestWindow::default();
		for format in vec![
			FileFormat::Png,
			FileFormat::Jpg,
			FileFormat::Bmp,
			FileFormat::Ico,
			FileFormat::Tiff,
			FileFormat::Tga,
			FileFormat::Pnm(String::from("ppm")),
			FileFormat::Ff,
		] {
			let path =
				FileUtil::get_path_with_extension(PathBuf::from("test.*"), &format);
			settings.save.file.format = format.clone();
			settings.save.file.path = path.clone();
			settings.analyze.file = path.clone();
			let app = App::new(Some(window), &settings);
			app.save_output((app.get_image(), None), File::create(&path)?);
			app.edit_image(&path);
			app.analyze_image()?;
			fs::remove_file(path)?;
		}
		settings.save.file.path = PathBuf::from("test");
		App::new(Some(window), &settings).start()?;
		fs::remove_file(settings.save.file.path)
	}
	#[test]
	fn test_app_anim() -> AppResult {
		let args = Args::default();
		let matches = ArgMatches::new(&args);
		let mut settings = AppSettings::new(&matches);
		settings.save.file.format = FileFormat::Gif;
		settings.record.command = Some("sleep 0.3");
		settings.anim.cut = (0.1, 0.1);
		let window = TestWindow::default();
		let app = App::new(Some(window), &settings);
		let images = app.get_frames().0;
		app.save_gif(Some((images.clone(), 10)), File::create("test.gif")?);
		app.edit_anim(File::open("test.gif")?, Path::new("test.gif"));
		let dir = env::current_dir()?;
		settings.split.dir = PathBuf::from(dir.to_str().unwrap_or_default());
		settings.split.file = PathBuf::from("test.gif");
		settings.save.file.format = FileFormat::Png;
		let app = App::new(Some(window), &settings);
		app.split_anim(File::open("test.gif")?)?;
		fs::remove_file("test.gif")?;
		app.save_apng(Some((images.clone(), 20)), File::create("test.apng")?);
		fs::remove_file("test.apng")?;
		for i in 0..images.len() {
			let path = PathBuf::from(format!("frame_{}.png", i));
			if path.exists() {
				fs::remove_file(path)?;
			}
		}
		Ok(())
	}
}
