use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use ash::vk::{Extent2D, Queue};
use log::{debug, error, info};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use crate::app::app::{AppConfig, UserEvent};
use crate::app::gui::{GuiSystem};
use crate::app::component::{ComponentRegistry};
use crate::app::Window;
use crate::graphics::pipeline_store::PipelineStore;
use crate::graphics::Renderer;
use crate::graphics::renderer::{WindowState};
use crate::vulkan::{Allocator, CommandBuffer, CommandPool, Device};

/**
 * ## Cen engine
 * Manages and connects all separate components.
 */
pub struct Engine {
    _start_time: SystemTime,
    window: Box<Window>,
    gui_system: Arc<Mutex<GuiSystem>>,
    renderer: Renderer,
    frame_count: usize,
    last_print_time: SystemTime,
    log_fps: bool,
    pub registry: ComponentRegistry,
}

pub struct InitContext<'a> {
    pub gui_system: &'a mut GuiSystem,
    pub device: &'a Device,
    pub allocator: &'a mut Allocator,
    pub pipeline_store: &'a mut PipelineStore,
    pub command_buffer: &'a mut CommandBuffer,
    pub swapchain_extent: Extent2D,
    pub queue: &'a Queue,
    pub command_pool: &'a CommandPool,
}
pub type InitCallback = Box<dyn FnOnce(&mut InitContext) -> ComponentRegistry>;

impl Engine {

    pub fn new(proxy: EventLoopProxy<UserEvent>, event_loop: &ActiveEventLoop, app_config: &AppConfig, init_callback: InitCallback) -> Engine {

        // Create the graphics context
        let window = Box::new(Window::create(event_loop, "cen", app_config.width, app_config.height, app_config.fullscreen, app_config.resizable));

        // Setup renderer
        let window_state = WindowState {
            window_handle: window.window_handle(),
            display_handle: window.display_handle(),
            extent2d: window.get_extent(),
            scale_factor: window.scale_factor(),
        };
        let mut renderer = Renderer::new(&window_state, proxy, app_config.vsync);

        // Setup gui
        let mut gui_system = GuiSystem::new(window.as_ref(), &mut renderer);

        // Initialize the user components
        let mut command_buffer = renderer.create_command_buffer();
        command_buffer.begin();
        let mut init_context = InitContext {
            gui_system: &mut gui_system,
            device: &renderer.device,
            allocator: &mut renderer.allocator,
            pipeline_store: &mut renderer.pipeline_store,
            command_buffer: &mut command_buffer,
            swapchain_extent: renderer.swapchain.get_extent(),
            queue: &renderer.queue,
            command_pool: &renderer.command_pool,
        };
        let registry: ComponentRegistry = init_callback(&mut init_context);
        command_buffer.end();
        renderer.submit_single_time_command_buffer(command_buffer);

        Engine {
            _start_time: SystemTime::now(),
            window,
            renderer,
            gui_system: Arc::new(Mutex::new(gui_system)),
            frame_count: 0,
            last_print_time: SystemTime::now(),
            registry,
            log_fps: app_config.log_fps,
        }
    }

    pub(crate) fn exit(&self) {
        // Wait for all render operations to finish before exiting
        // This ensures we can safely start dropping gpu resources
        self.renderer.device.wait_idle();
    }
    
    pub(crate) fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        self.window.window_event( event.clone(), event_loop );

        self.gui_system.lock().unwrap().on_window_event(self.window.winit_window(), &event);

        match event {
            WindowEvent::RedrawRequested => {
                self.draw();

                if self.log_fps {
                    let current_frame_time = SystemTime::now();
                    let elapsed = current_frame_time.duration_since(self.last_print_time).unwrap();
                    self.frame_count += 1;

                    if elapsed.as_secs() >= 1 {
                        info!("fps: {}, frametime: {:.3}ms", self.frame_count, elapsed.as_millis() as f32 / self.frame_count as f32);
                        self.frame_count = 0;
                        self.last_print_time = current_frame_time;
                    }
                }
            },
            WindowEvent::Resized( .. ) => {
                let window_state = WindowState {
                    window_handle: self.window.window_handle(),
                    display_handle: self.window.display_handle(),
                    extent2d: self.window.get_extent(),
                    scale_factor: self.window.scale_factor(),
                };
                self.renderer.recreate_window(window_state);
            },
            WindowEvent::ScaleFactorChanged {  .. } => {
                let window_state = WindowState {
                    window_handle: self.window.window_handle(),
                    display_handle: self.window.display_handle(),
                    extent2d: self.window.get_extent(),
                    scale_factor: self.window.scale_factor(),
                };
                self.renderer.recreate_window(window_state);
            }
            _ => (),
        }
    }

    pub fn user_event(&mut self, _: &ActiveEventLoop, event: UserEvent) {
        match event {
            | UserEvent::GlslUpdate(path) => {
                debug!("Reloading shader: {:?}", path);

                if let Err(e) = self.renderer.pipeline_store.reload(&path) {
                    error!("{}", e);
                }
            }
            _ => (),
        }
    }
    
    pub fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        match cause {
            | StartCause::Poll => {
                self.update();
                self.window.winit_window().request_redraw();
            }
            _ => {}
        }
    }

    fn update(&mut self) {

    }
    
    pub fn draw(&mut self) {
        
        // Update our gui. Has to happen each frame or we will miss frames
        self.gui_system.lock().unwrap().update(
            self.window.winit_window(),
            self.registry.gui_components().as_mut_slice()
        );

        // Render all our components
        let mut draw_components = self.registry.render_components();
        // Add our gui system to our render components
        draw_components.push(self.gui_system.clone());
        self.renderer.draw_frame(
            draw_components.as_slice()
        );
    }
}
