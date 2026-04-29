use ash::vk::Extent2D;
use winit::raw_window_handle::{DisplayHandle, WindowHandle};

pub struct WindowState<'a> {
    pub window_handle: WindowHandle<'a>,
    pub display_handle: DisplayHandle<'a>,
    pub extent2d: Extent2D,
    pub scale_factor: f64,
}
