use std::sync::{Arc, Mutex};
use cen::app::app::{App, AppConfig};
use cen::graphics::{Renderer};
use cen::graphics::renderer::{RenderComponent, RenderContext};

struct EmptyRend {
}

impl RenderComponent for EmptyRend {
    fn initialize(&mut self, _: &mut Renderer) {
    }

    fn render(&mut self, _: &mut RenderContext) {
    }
}

fn main() {
    App::run(
        AppConfig::default(),
        Arc::new(Mutex::new(EmptyRend {})),
        None
    );
}
