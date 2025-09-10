use std::ops::{DerefMut};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use log::{debug, error, info};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use crate::app::app::{AppConfig, UserEvent};
use crate::app::gui::{GuiComponent, GuiSystem};
use crate::app::Window;
use crate::graphics::Renderer;
use crate::graphics::renderer::{RenderComponent, WindowState};

pub struct Engine {
    _start_time: SystemTime,
    window: Box<Window>,
    component: Arc<Mutex<dyn RenderComponent>>,
    gui: Option<Arc<Mutex<dyn GuiComponent>>>,
    gui_system: GuiSystem,
    renderer: Renderer,
    frame_count: usize,
    last_print_time: SystemTime,
    log_fps: bool,
}

impl Engine {
    pub(crate) fn exit(&self) {
        // Wait for all render operations to finish before exiting
        // This ensures we can safely start dropping gpu resources
        self.renderer.device.wait_idle();
    }
    
    pub(crate) fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        self.window.window_event( event.clone(), event_loop );

        self.gui_system.on_window_event(self.window.winit_window(), &event);

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
    
    pub fn new(proxy: EventLoopProxy<UserEvent>, event_loop: &ActiveEventLoop, app_config: &AppConfig, user_component: Arc<Mutex<dyn RenderComponent>>, gui_component: Option<Arc<Mutex<dyn GuiComponent>>>) -> Engine {
        // Create the graphics context
        let window = Box::new(Window::create(&event_loop, "cen", app_config.width, app_config.height, app_config.fullscreen, app_config.resizable));

        // Setup renderer
        let window_state = WindowState {
            window_handle: window.window_handle(),
            display_handle: window.display_handle(),
            extent2d: window.get_extent(),
            scale_factor: window.scale_factor(),
        };

        let mut renderer = Renderer::new(&window_state, proxy, app_config.vsync);

        user_component.lock().unwrap().initialize(&mut renderer);

        // Initialize gui renderer
        let mut gui_system = GuiSystem::new(window.as_ref());
        gui_system.initialize(&mut renderer);
        
        // Initialize gui component
        if let Some(gui) = gui_component.as_ref() {
            gui.lock().as_mut().unwrap().initialize_gui(&mut gui_system);
        }

        Engine {
            _start_time: SystemTime::now(),
            window,
            renderer,
            gui_system,
            frame_count: 0,
            last_print_time: SystemTime::now(),
            component: user_component,
            log_fps: app_config.log_fps,
            gui: gui_component,
        }
    }
    
    pub fn update(&mut self) {
    }
    
    pub fn draw(&mut self) {
        
        // Update our gui. Has to happen each frame or we will miss frames
        if let Some(gui) = &self.gui {
            self.gui_system.update(
                self.window.winit_window(),
                &mut [gui.lock().unwrap().deref_mut()]
            );
        }
        
        self.renderer.update();
        
        // Render all our components
        self.renderer.draw_frame(&mut [
            self.component.lock().unwrap().deref_mut(),
            &mut self.gui_system
        ]);
    }
}
