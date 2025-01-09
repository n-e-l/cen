use ash::vk;
use cen::app::app::{App, AppConfig};
use cen::graphics::{Renderer};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::CommandBuffer;

struct EmptyRend {
}

impl RenderComponent for EmptyRend {
    fn initialize(&mut self, _: &mut Renderer) {
    }

    fn render(&mut self, _: &mut Renderer, _: &mut CommandBuffer, _: &vk::Image, _: &vk::ImageView) {
    }
}

fn main() {
    App::run(AppConfig::default(), Box::new(EmptyRend {
    }));
}
