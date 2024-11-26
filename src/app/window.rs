use ash::vk::Extent2D;
use winit::event::WindowEvent;
use winit::event::{ElementState, KeyEvent};
use winit::event_loop::{ActiveEventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle, WindowHandle};
use winit::window::WindowAttributes;

/// System window wrapper.
/// Handles window events i.e. close, redraw, keyboard input.
pub struct Window {
    window: winit::window::Window,
}

impl Window {
    pub fn create(event_loop: &ActiveEventLoop, window_title: &str, width: u32, height: u32, fullscreen: bool) -> Window {
        let mut attributes = WindowAttributes::default()
            .with_title(window_title)
            .with_resizable(false)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height));

        if fullscreen {
            attributes = attributes.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }

        let mut window = event_loop.create_window(attributes).expect("Failed to create window");

        Window {
            window,
        }
    }

    pub fn winit_window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn window_handle(&self) -> WindowHandle {
        self.window.window_handle().unwrap()
    }

    pub fn display_handle(&self) -> DisplayHandle {
        self.window.display_handle().unwrap()
    }

    pub fn get_extent(&self) -> Extent2D {
        let width = self.window.inner_size().width;
        let height = self.window.inner_size().height;
        Extent2D{ width, height }
    }

    pub fn window_event(&mut self, event: WindowEvent, event_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key.as_ref() {
                Key::Named(NamedKey::Escape) => {
                    event_loop.exit();
                },
                Key::Character("q") => {
                    event_loop.exit();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
