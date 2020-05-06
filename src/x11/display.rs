use crate::x11::window::Window;
use std::ptr;
use x11::xlib;

/* X11 display */
pub struct Display {
	display: *mut xlib::Display,
}

impl Display {
	/**
	 * Open a display.
	 *
	 * @return Display (Option)
	 */
	pub fn open() -> Option<Self> {
		let display = unsafe { xlib::XOpenDisplay(ptr::null()) };
		if !display.is_null() {
			Some(Self { display })
		} else {
			None
		}
	}

	pub fn get(&self) -> *mut xlib::Display {
		self.display
	}

	/**
	 * Get the root window of the default screen.
	 *
	 * @return Window
	 */
	#[allow(dead_code)]
	pub fn get_root_window(&self) -> Window {
		let root_window: usize;
		unsafe {
			let screen = xlib::XDefaultScreenOfDisplay(self.display);
			root_window = xlib::XRootWindowOfScreen(screen) as usize;
		};
		Window::new(root_window as u64, self.display)
	}

	/**
	 * Get the focused window.
	 *
	 * @return Window
	 */
	pub fn get_focused_window(&self) -> Window {
		let focus_window: *mut xlib::Window = &mut 0;
		let revert_to_return: *mut i32 = &mut 0;
		unsafe {
			xlib::XGetInputFocus(self.display, focus_window, revert_to_return);
		};
		Window::new(unsafe { *focus_window }, self.display)
	}
}

/* Close the display when the Display object went out of scope. */
impl Drop for Display {
	fn drop(&mut self) {
		unsafe {
			xlib::XCloseDisplay(self.display);
		}
	}
}
