use ash::vk::Image;
use cen::app::app::{App, AppConfig};
use cen::graphics::{Renderer};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::CommandBuffer;

struct EmptyRend {
}

impl RenderComponent for EmptyRend {
    fn initialize(&mut self, renderer: &mut Renderer) {
    }

    fn render(&mut self, _: &mut Renderer, _: &mut CommandBuffer, _: &Image) {
    }
}

fn main() {
    App::run(AppConfig::default(), Box::new(EmptyRend{}));
}