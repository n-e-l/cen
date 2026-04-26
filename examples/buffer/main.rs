use ash::vk;
use ash::vk::{BufferImageCopy, BufferUsageFlags, DeviceSize, Extent3D, ImageLayout, ImageSubresourceLayers};
use egui::Context;
use gpu_allocator::MemoryLocation;
use winit::event::WindowEvent;
use cen::app::Cen;
use cen::app::app::{AppComponent, AppConfig};
use cen::app::engine::InitContext;
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::graphics::renderer::{RenderComponent, RenderContext};
use cen::vulkan::{Buffer, ImageTrait};

struct BufferExample {
    buffer: Buffer,
}

impl AppComponent for BufferExample {
    fn new(ctx: &mut InitContext) -> Self {
        // Image
        let buffer = Buffer::new(
            &ctx.device,
            &mut ctx.allocator,
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

        Self {
            buffer
        }
    }

    fn window_event(&mut self, _: WindowEvent) {
    }
}

impl RenderComponent for BufferExample {

    fn render(&mut self, ctx: &mut RenderContext) {

        if self.buffer.size() != (ctx.swapchain_image.width() * ctx.swapchain_image.height() * 4) as u64 {
            // Recreate image
            self.buffer = Buffer::new(
                &ctx.device,
                &mut ctx.allocator,
                MemoryLocation::CpuToGpu,
                (ctx.swapchain_image.width() * ctx.swapchain_image.height() * 4) as DeviceSize,
                BufferUsageFlags::TRANSFER_SRC
            );

            {
                let mut mem = self.buffer.mapped().unwrap();
                for i in mem.as_mut_slice() {
                    *i = 255u8;
                }
            }
        }

        // Transition the swapchain image
        ctx.command_buffer.image_barrier(
            ctx.swapchain_image,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::NONE,
            vk::AccessFlags::TRANSFER_WRITE
        );

        // Copy to the swapchain
        ctx.command_buffer.clear_color_image(
            ctx.swapchain_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            [1.0, 0.0, 0.0, 1.0]
        );

        ctx.command_buffer.copy_buffer_to_image(
            &self.buffer,
            ctx.swapchain_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &[
                BufferImageCopy::default()
                    .buffer_image_height(0)
                    .buffer_row_length(0)
                    .buffer_offset(0)
                    .image_extent(
                        Extent3D::default()
                            .width(ctx.swapchain_image.width())
                            .height(ctx.swapchain_image.height())
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

        // Transfer back to default states
        ctx.command_buffer.image_barrier(
            ctx.swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::NONE
        );

    }
}

impl GuiComponent for BufferExample {
    fn gui(&mut self, _: &mut GuiHandler, _: &Context) {}
}

fn main() {
    Cen::<BufferExample>::run(AppConfig::default());
}

