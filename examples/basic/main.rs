use ash::vk::Image;
use cen::app::app::{App, AppConfig};
use cen::graphics::{Renderer};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::CommandBuffer;

struct BasicRend {
}

impl RenderComponent for BasicRend {
    fn render(&self, _: &mut Renderer, _: &mut CommandBuffer, _: &Image) {
    }
}

fn main() {

    let app = App::new(AppConfig {
        width: 1000,
        height: 1000,
        vsync: true,
        log_fps: false,
    });

    let render_comp = BasicRend {};
    app.run(Box::new(render_comp));
}