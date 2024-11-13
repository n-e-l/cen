use winit::application::ApplicationHandler;
use std::path::{PathBuf};
use std::time::{SystemTime};
use env_logger::{Builder, Env};
use log::{debug, error, info, LevelFilter};
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopBuilder, EventLoopProxy};
use winit::window::WindowId;
use crate::app::{Window};
use crate::graphics::Renderer;
use crate::graphics::renderer::{RenderComponent, WindowState};

pub struct App<T>
where T: RenderComponent
{
    initialized: bool,
    _start_time: SystemTime,
    components: Option<T>,
    renderer: Option<Renderer>,
    window: Option<Window>,
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
}

impl AppConfig {

    pub fn default() -> Self {
        Self {
            width: 1000,
            height: 1000,
            vsync: true,
            log_fps: false
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
}

#[derive(Debug, Default)]
pub enum UserEvent {
    #[default]
    None,
    GlslUpdate(PathBuf),
}

impl<T: RenderComponent> ApplicationHandler<UserEvent> for App<T>
{
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        match cause {
            | StartCause::Poll => {
                self.renderer.as_mut().unwrap().draw_frame(self.components.as_mut().unwrap());

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
        let window = Window::create(&event_loop, "cen", self.app_config.width, self.app_config.height);
        let window_state = WindowState {
            window_handle: window.window_handle(),
            display_handle: window.display_handle(),
            extent2d: window.get_extent()
        };

        let mut renderer = Renderer::new(&window_state, self.proxy.clone(), self.app_config.vsync);

        self.components = Some(T::construct(&mut renderer));
        // self.gui_component = Some(GuiComponent::new(&self.renderer.as_mut().unwrap(), &window.display_handle()));

        self.renderer = Some(renderer);

        self.window = Some(window);
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
        self.window.as_mut().unwrap().window_event( event.clone(), event_loop );

        match event {
            WindowEvent::RedrawRequested => {
                self.renderer.as_mut().unwrap().draw_frame(self.components.as_mut().unwrap());
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

impl<T: RenderComponent> App<T> {

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
            .init();
    }

    pub fn run(app_config: AppConfig) {

        Self::init_logger();

        // App setup
        let start_time = SystemTime::now();

        let event_loop = EventLoopBuilder::default().build().expect("Failed to create event loop.");

        let mut app: App<T> = App {
            initialized: false,
            window: None,
            renderer: None,
            components: None,
            // gui_component: None,
            _start_time: start_time,
            frame_count: 0,
            app_config,
            last_print_time: SystemTime::now(),
            proxy: event_loop.create_proxy(),
        };

        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut app).unwrap();
    }

}