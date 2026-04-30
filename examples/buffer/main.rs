use ash::vk;
use ash::vk::{BufferImageCopy, BufferUsageFlags, DeviceSize, Extent3D, ImageLayout, ImageSubresourceLayers};
use egui::Context;
use gpu_allocator::MemoryLocation;
use winit::event::WindowEvent;
use cen::app::app::{AppComponent, AppConfig, Cen};
use cen::app::engine::CenContext;
use cen::app::gui::{GuiComponent, GuiContext};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, ImageTrait};

struct BufferExample {
    buffer: Buffer,
}

impl AppComponent for BufferExample {
    fn new(ctx: &mut CenContext) -> Self {
        let buffer = Buffer::new(
            &ctx.gfx.device,
            &mut ctx.gfx.allocator,
            MemoryLocation::CpuToGpu,
            2000 * 2000 * 4,
            BufferUsageFlags::TRANSFER_SRC
        );

        {
            let mut mem = buffer.mapped().unwrap();
            for i in mem.as_mut_slice() {
                *i = 255u8;
            }
        }

        Self { buffer }
    }

    fn window_event(&mut self, _: WindowEvent) {}
}

impl RenderComponent for BufferExample {
    fn render(&mut self, ctx: &mut CenContext) {
        let swapchain_image = ctx.swapchain_image.unwrap();

        if self.buffer.size() != (swapchain_image.width() * swapchain_image.height() * 4) as u64 {
            self.buffer = Buffer::new(
                &ctx.gfx.device,
                &mut ctx.gfx.allocator,
                MemoryLocation::CpuToGpu,
                (swapchain_image.width() * swapchain_image.height() * 4) as DeviceSize,
                BufferUsageFlags::TRANSFER_SRC
            );

            {
                let mut mem = self.buffer.mapped().unwrap();
                for i in mem.as_mut_slice() {
                    *i = 255u8;
                }
            }
        }

        ctx.command_buffer.transition(swapchain_image, vk::ImageLayout::PRESENT_SRC_KHR, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        ctx.command_buffer.clear_color_image(
            swapchain_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            [1.0, 0.0, 0.0, 1.0]
        );

        ctx.command_buffer.copy_buffer_to_image(
            &self.buffer,
            swapchain_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &[
                BufferImageCopy::default()
                    .buffer_image_height(0)
                    .buffer_row_length(0)
                    .buffer_offset(0)
                    .image_extent(
                        Extent3D::default()
                            .width(swapchain_image.width())
                            .height(swapchain_image.height())
                            .depth(1)
                    )
                    .image_subresource(
                        ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0)
                    )
            ]
        );

        ctx.command_buffer.transition(swapchain_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR);
    }
}

impl GuiComponent for BufferExample {
    fn gui(&mut self, _: &mut GuiContext, _: &Context) {}
}

fn main() {
    Cen::<BufferExample>::run(AppConfig::default());
}
