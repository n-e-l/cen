use std::time::SystemTime;
use log::{debug, error, info};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use crate::app::app::{AppComponent, AppConfig, UserEvent};
use crate::app::gui::{GuiComponent, GuiSystem};
use crate::app::{Window};
use crate::graphics::{Renderer};
use crate::graphics::{GraphicsContext, ImageContext, PipelineContext};
use crate::graphics::renderer::RenderComponent;
use crate::vulkan::WindowState;
use crate::vulkan::{CommandBuffer, ImageTrait, SwapchainImage};

/**
 * Cen engine
 * Manages and connects all separate components.
 */
pub struct Engine {
    _start_time: SystemTime,
    window: Box<Window>,
    gui_system: GuiSystem,
    renderer: Renderer,
    frame_count: usize,
    last_print_time: SystemTime,
    log_fps: bool,
    app_component: Box<dyn AppComponent>
}

pub struct CenContext<'a>
{
    pub gfx: &'a mut GraphicsContext,
    pub images: &'a mut ImageContext,
    pub pipelines: &'a mut PipelineContext,
    pub command_buffer: &'a mut CommandBuffer,
    pub swapchain_image: Option<&'a SwapchainImage>,
}

impl Engine {

    pub fn new<C: AppComponent + 'static>(proxy: EventLoopProxy<UserEvent>, event_loop: &ActiveEventLoop, app_config: &AppConfig) -> Engine {

        // Create the graphics context
        let window = Box::new(Window::create(event_loop, &app_config.title, app_config.width, app_config.height, app_config.fullscreen, app_config.resizable));

        // Setup renderer
        let window_state = WindowState {
            window_handle: window.window_handle(),
            display_handle: window.display_handle(),
            extent2d: window.get_extent(),
            scale_factor: window.scale_factor(),
        };
        let mut renderer = Renderer::new(&window_state, proxy, app_config.vsync);

        // Setup gui
        let gui_system = GuiSystem::new(window.as_ref(), &mut renderer);


        // Initialize the user components
        let mut command_buffer = CommandBuffer::new(&renderer.graphics_context.device, &renderer.graphics_context.command_pool, false);
        command_buffer.begin();

        let mut init_context = CenContext {
            gfx: &mut renderer.graphics_context,
            images: &mut renderer.image_context,
            pipelines: &mut renderer.pipeline_context,
            command_buffer: &mut command_buffer,
            swapchain_image: None,
        };
        let app_component = Box::new(C::new(&mut init_context));

        command_buffer.end();
        renderer.submit_single_time_command_buffer(command_buffer);

        Engine {
            _start_time: SystemTime::now(),
            window,
            renderer,
            gui_system,
            frame_count: 0,
            app_component,
            last_print_time: SystemTime::now(),
            log_fps: app_config.log_fps,
        }
    }

    pub(crate) fn exit(&self) {
        // Wait for all render operations to finish before exiting
        // This ensures we can safely start dropping gpu resources
        self.renderer.graphics_context.device.wait_idle();
    }
    
    pub(crate) fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        self.window.window_event( event.clone(), event_loop );

        self.gui_system.on_window_event(self.window.winit_window(), &event);

        self.app_component.window_event( event.clone());

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
                self.renderer.on_window_recreation(&mut self.gui_system.gui_data, window_state);
            },
            WindowEvent::ScaleFactorChanged { .. } => {
                let window_state = WindowState {
                    window_handle: self.window.window_handle(),
                    display_handle: self.window.display_handle(),
                    extent2d: self.window.get_extent(),
                    scale_factor: self.window.scale_factor(),
                };
                self.renderer.on_window_recreation(&mut self.gui_system.gui_data, window_state);
            }
            _ => (),
        }
    }

    pub fn user_event(&mut self, _: &ActiveEventLoop, event: UserEvent) {
        match event {
            | UserEvent::GlslUpdate(path) => {
                debug!("Reloading shader: {:?}", path);

                if let Err(e) = self.renderer.pipeline_context.pipeline_store.reload(&path) {
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
        let mut gui_components: Vec<&mut dyn GuiComponent> = vec![self.app_component.as_mut()];
        self.gui_system.update(
            &mut self.renderer.graphics_context,
            &mut self.renderer.image_context,
            self.window.winit_window(),
            &mut gui_components
        );

        // Render all our components
        let mut render_components: Vec<&mut dyn RenderComponent> = vec![self.app_component.as_mut()];
        self.renderer.draw_frame(&mut self.gui_system, &mut render_components);
    }
}
