use std::thread;
use std::time::{Duration, Instant};

/* FPS controller */
#[derive(Clone, Copy, Debug)]
pub struct FpsClock {
	pub fps: u32,
	last_tick_time: Instant,
}

impl FpsClock {
	/**
	 * Create a new FpsClock object.
	 *
	 * @param  fps
	 * @return FpsClock
	 */
	pub fn new(fps: u32) -> Self {
		Self {
			fps,
			last_tick_time: Instant::now(),
		}
	}

	/**
	 * Sleep the thread to run at the correct FPS.
	 *
	 * @return f32
	 */
	pub fn tick(&mut self) -> f32 {
		let t = self.last_tick_time.elapsed();
		let total_nanos = t.as_secs() * 1e9 as u64 + t.subsec_nanos() as u64;
		let diff = ((1. / self.fps as f32) * 1e9) - (total_nanos as f32);
		if diff > 0. {
			thread::sleep(Duration::new(0, diff as u32))
		}
		self.last_tick_time = Instant::now();
		diff
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pretty_assertions::assert_eq;
	use std::thread;
	#[test]
	fn test_fps() {
		let mut fps_clock = FpsClock::new(100);
		let fps = (1. / fps_clock.fps as f32) * 1e9;
		assert_eq!(1e7, fps);
		for i in 0..2 {
			thread::sleep(Duration::from_nanos(i));
			assert!(fps > fps_clock.tick());
		}
	}
}
