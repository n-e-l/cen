use winit::application::ApplicationHandler;
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};
use env_logger::{Builder, Env};
use log::{LevelFilter};
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use winit::window::WindowId;
use crate::app::engine::Engine;
use crate::app::gui::GuiComponent;
use crate::graphics::renderer::{RenderComponent};

pub struct App
{
    pub proxy: EventLoopProxy<UserEvent>,
    pub app_config: AppConfig,
    pub render_component: Option<Arc<Mutex<dyn RenderComponent>>>,
    pub gui_component: Option<Arc<Mutex<dyn GuiComponent>>>,
    engine: Option<Engine>,
}

pub struct AppConfig {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) vsync: bool,
    pub(crate) log_fps: bool,
    pub(crate) fullscreen: bool,
    pub(crate) resizable: bool,
}

impl AppConfig {

    pub fn default() -> Self {
        Self {
            width: 1000,
            height: 1000,
            vsync: true,
            log_fps: false,
            fullscreen: false,
            resizable: false,
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

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
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
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let Some(ref mut engine) = self.engine {
            engine.new_events(event_loop, cause);
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {

        // Prepare for multiple resume calls
        if let None = self.engine {
            self.engine = Some(Engine::new(
                self.proxy.clone(),
                event_loop,
                &self.app_config,
                self.render_component.take().unwrap(),
                self.gui_component.take()
            ));
        }

    }

    fn user_event(&mut self, even_loop: &ActiveEventLoop, event: UserEvent) {
        if let Some(engine) = self.engine.as_mut() {
            engine.user_event(even_loop, event);
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(engine) = self.engine.as_mut() {
            engine.window_event(event_loop, event);
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(engine) = self.engine.take() {
            engine.exit();
        }
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
            .filter(Some("egui_winit"), LevelFilter::Error)
            .init();
    }
    
    fn new(app_config: AppConfig, event_loop: &EventLoop<UserEvent>, render_component: Arc<Mutex<dyn RenderComponent>>, gui_component: Option<Arc<Mutex<dyn GuiComponent>>>) -> Self {
        
        let proxy = event_loop.create_proxy();
        
        App {
            app_config,
            proxy,
            render_component: Some(render_component),
            gui_component,
            engine: None,
        }
    }
    
    pub fn run(app_config: AppConfig, render_component: Arc<Mutex<dyn RenderComponent>>, gui_component: Option<Arc<Mutex<dyn GuiComponent>>>) {

        Self::init_logger();

        let event_loop = EventLoopBuilder::default().build().expect("Failed to create event loop.");
        event_loop.set_control_flow(ControlFlow::Poll);

        // App setup
        let mut app = App::new(app_config, &event_loop, render_component, gui_component);
        event_loop.run_app(&mut app).unwrap();
    }

}
