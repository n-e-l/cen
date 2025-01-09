use winit::application::ApplicationHandler;
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime};
use ash::vk::Extent2D;
use env_logger::{Builder, Env};
use log::{debug, error, info, LevelFilter};
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopBuilder, EventLoopProxy};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::WindowId;
use crate::app::{Window};
use crate::app::gui::GuiComponent;
use crate::graphics::Renderer;
use crate::graphics::renderer::{RenderComponent, WindowState};

pub struct App
{
    initialized: bool,
    _start_time: SystemTime,
    component: Box<dyn RenderComponent>,
    gui_component: Option<GuiComponent>,
    renderer: Option<Renderer>,
    window: Option<Arc<Mutex<Window>>>,
    frame_count: usize,
    pub app_config: AppConfig,
    last_print_time: SystemTime,
    pub proxy: EventLoopProxy<UserEvent>,
}

pub struct AppConfig {
    width: u32,
    height: u32,
    vsync: bool,
    log_fps: bool,
    fullscreen: bool,
}

impl AppConfig {

    pub fn default() -> Self {
        Self {
            width: 1000,
            height: 1000,
            vsync: true,
            log_fps: false,
            fullscreen: false
        }
    }

    pub fn width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn log_fps(mut self, log_fps: bool) -> Self {
        self.log_fps = log_fps;
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

}

#[derive(Debug, Default)]
pub enum UserEvent {
    #[default]
    None,
    GlslUpdate(PathBuf),
}

impl ApplicationHandler<UserEvent> for App
{
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        match cause {
            | StartCause::Poll => {
                self.draw();

                if self.app_config.log_fps {
                    let current_frame_time = SystemTime::now();
                    let elapsed = current_frame_time.duration_since(self.last_print_time).unwrap();
                    self.frame_count += 1;

                    if elapsed.as_secs() >= 1 {
                        info!("fps: {}, frametime: {:.3}ms", self.frame_count, elapsed.as_millis() as f32 / self.frame_count as f32);
                        self.frame_count = 0;
                        self.last_print_time = current_frame_time;
                    }
                }
            }
            _ => {}
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {

        // Prepare for multiple resume calls
        if self.initialized {
            return;
        }
        self.initialized = true;

        // Create the graphics context
        let window = Window::create(&event_loop, "cen", self.app_config.width, self.app_config.height, self.app_config.fullscreen);

        // Setup renderer
        {
            let window_state = WindowState {
                window_handle: window.window_handle(),
                display_handle: window.display_handle(),
                extent2d: window.get_extent(),
            };
            
            let renderer = Renderer::new(&window_state, self.proxy.clone(), self.app_config.vsync);
            
            self.renderer = Some(renderer);
        }

        self.window = Some(Arc::new(Mutex::new(window)));

        // Add gui component (wip)
        self.component.initialize(self.renderer.as_mut().unwrap());
        self.gui_component = Some(GuiComponent::new(Arc::downgrade(&self.window.as_mut().unwrap())));
        self.gui_component.as_mut().unwrap().initialize(self.renderer.as_mut().unwrap());
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: UserEvent) {
        match event {
            | UserEvent::GlslUpdate(path) => {
                debug!("Reloading shader: {:?}", path);

                if let Err(e) = self.renderer.as_mut().unwrap().pipeline_store.reload(&path) {
                    error!("{}", e);
                }
            }
            _ => (),
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.window.as_mut().unwrap().lock().unwrap().window_event( event.clone(), event_loop );
        
        self.gui_component.as_mut().unwrap().on_window_event(self.window.as_mut().unwrap().lock().unwrap().winit_window(), &event);

        match event {
            WindowEvent::RedrawRequested => {
                self.draw();
            },
            WindowEvent::Resized( _ ) => {
            }
            _ => (),
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {

        // Wait for all render operations to finish before exiting
        // This ensures we can safely start dropping gpu resources
        self.renderer.as_mut().unwrap().device.wait_idle();
    }

    fn memory_warning(&mut self, _: &ActiveEventLoop) {
    }
}

impl App {

    fn init_logger() {
        let env = Env::default()
            .filter_or("LOG_LEVEL", "trace")
            .write_style_or("LOG_STYLE", "always");

        Builder::from_env(env)
            .format_level(true)
            .format_timestamp_millis()
            .filter(Some("winit"), LevelFilter::Error)
            .filter(Some("calloop"), LevelFilter::Error)
            .filter(Some("notify::inotify"), LevelFilter::Error)
            .filter(Some("mio::poll"), LevelFilter::Error)
            .filter(Some("sctk"), LevelFilter::Error)
            .filter(Some("notify_debouncer_mini"), LevelFilter::Error)
            .filter(Some("egui_ash_renderer"), LevelFilter::Error)
            .init();
    }

    pub fn run(app_config: AppConfig, render_component: Box<dyn RenderComponent>) {

        Self::init_logger();

        // App setup
        let start_time = SystemTime::now();

        let event_loop = EventLoopBuilder::default().build().expect("Failed to create event loop.");


        let mut app: App = App {
            initialized: false,
            window: None,
            renderer: None,
            component: render_component,
            gui_component: None,
            _start_time: start_time,
            frame_count: 0,
            app_config,
            last_print_time: SystemTime::now(),
            proxy: event_loop.create_proxy(),
        };

        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut app).unwrap();
    }

    fn draw(&mut self) {
        self.renderer.as_mut().unwrap().draw_frame(&mut [
            self.component.as_mut(),
            self.gui_component.as_mut().unwrap()
        ]);
    }

}