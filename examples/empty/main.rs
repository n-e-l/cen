use ash::vk::Image;
use cen::app::app::{App, AppConfig};
use cen::graphics::{Renderer};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::CommandBuffer;

struct EmptyRend {
}

impl RenderComponent for EmptyRend {
    fn render(&mut self, _: &mut Renderer, _: &mut CommandBuffer, _: &Image) {
    }
}

fn main() {

    let mut app = App::new(AppConfig {
        width: 1000,
        height: 1000,
        vsync: true,
        log_fps: false,
    });

    let mut render_comp = EmptyRend {};
    app.run(&mut render_comp, None);
}