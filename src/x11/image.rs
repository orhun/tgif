#[derive(Debug)]
pub struct Bgr {
	b: u8,
	g: u8,
	r: u8,
	_p: u8,
}

impl Bgr {
	pub fn get_rgb_pixels(bgr_data: &[Bgr]) -> Vec<u8> {
		let mut pixels = Vec::new();
		for bgr in bgr_data {
			pixels.extend(&[bgr.r, bgr.g, bgr.b])
		}
		pixels
	}
}
