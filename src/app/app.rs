use std::path::{PathBuf};
use std::time::{Duration, SystemTime};
use env_logger::{Builder, Env};
use log::{debug, error, info, LevelFilter};
use notify_debouncer_mini::DebouncedEventKind::Any;
use notify_debouncer_mini::DebounceEventResult;
use winit::event::{Event, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use crate::app::{Window};
use crate::graphics::Renderer;
use crate::graphics::renderer::{RenderComponent, WindowState};

pub struct App {
    _start_time: SystemTime,
    renderer: Renderer,
    window: Window,
    event_loop: EventLoop<UserEvent>,
    pub app_config: AppConfig,
}

pub struct AppConfig {
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub log_fps: bool,
}

#[derive(Debug)]
pub enum UserEvent {
    GlslUpdate(PathBuf),
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
            .init();
    }

    pub fn new(app_config: AppConfig) -> App{

        Self::init_logger();

        // App setup
        let start_time = SystemTime::now();

        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build().expect("Failed to create event loop.");
        let window = Window::create(&event_loop, "cen", app_config.width, app_config.height);
        let window_state = WindowState {
            window_handle: window.window_handle(),
            display_handle: window.display_handle(),
            extent2d: window.get_extent()
        };
        let renderer = Renderer::new(&window_state, event_loop.create_proxy(), app_config.vsync);

        App {
            event_loop,
            window,
            renderer,
            _start_time: start_time,
            app_config,
        }
    }

    fn watch_callback(event_loop_proxy: EventLoopProxy<UserEvent>) -> impl FnMut(DebounceEventResult) {
        move |event| match event {
            Ok(events) => {
                if let Some(e) = events
                    .iter()
                    .filter(|e| e.kind == Any)
                    .next()
                {
                    event_loop_proxy.send_event(
                        UserEvent::GlslUpdate(e.path.clone())
                    ).expect("Failed to send event")
                }
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }

    pub fn run(&mut self, component: &dyn RenderComponent, mut update: Option<&mut dyn FnMut()>) {

        // Register file watching for the shaders
        let _watcher = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(250),
            Self::watch_callback(self.event_loop.create_proxy())
        ).expect("Failed to create file watcher");

        // TODO: Watch created shaders
        /*let paths = &draw_config.passes.iter().map(|p| { p.shader.clone() }).collect::<Vec<String>>();
        for path in paths {
            watcher.watcher().watch(Path::new(path), RecursiveMode::Recursive).unwrap();
        }*/

        // Event loop

        let mut last_print_time = SystemTime::now();
        let mut frame_count = 0;
        self.event_loop
            .run_on_demand( |event, elwt| {
                elwt.set_control_flow(ControlFlow::Poll);

                match event {
                    | Event::NewEvents(StartCause::Poll) => {
                        if let Some(u) = update.as_mut() {
                            u();
                        }

                        self.renderer.draw_frame(component);

                        if self.app_config.log_fps {
                            let current_frame_time = SystemTime::now();
                            let elapsed = current_frame_time.duration_since(last_print_time).unwrap();
                            frame_count += 1;

                            if elapsed.as_secs() >= 1 {
                                info!("fps: {}, frametime: {:.3}ms", frame_count, elapsed.as_millis() as f32 / frame_count as f32);
                                frame_count = 0;
                                last_print_time = current_frame_time;
                            }
                        }
                    }
                    | Event::WindowEvent { event, .. } => {
                        self.window.window_event( event.clone(), elwt );

                        match event {
                            WindowEvent::RedrawRequested => {
                                self.renderer.draw_frame(component);
                            },
                            WindowEvent::Resized( _ ) => {
                            }
                            _ => (),
                        }
                    }
                    | Event::UserEvent( UserEvent::GlslUpdate(path) ) => {
                        debug!("Reloading shader: {:?}", path);

                        if let Err(e) = self.renderer.pipeline_store.reload(&path) {
                            error!("{}", e);
                        }
                    }
                    _ => (),
                }

            })
            .unwrap();

        // Wait for all render operations to finish before exiting
        // This ensures we can safely start dropping gpu resources
        self.renderer.device.wait_idle();
    }

    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn window(&mut self) -> &mut Window {
        &mut self.window
    }
}