use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::{BufferImageCopy, BufferUsageFlags, Extent3D, ImageLayout, ImageSubresourceLayers};
use gpu_allocator::MemoryLocation;
use cen::app::App;
use cen::app::app::AppConfig;
use cen::graphics::Renderer;
use cen::graphics::renderer::{RenderComponent, RenderContext};
use cen::vulkan::{Buffer};

struct ComputeRender {
    buffer: Option<Buffer>,
}

impl RenderComponent for ComputeRender {
    fn initialize(&mut self, renderer: &mut Renderer) {
        // Image
        let buffer = Buffer::new(
            &renderer.device,
            &mut renderer.allocator,
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

        self.buffer = Some(buffer);
    }

    fn render(&mut self, ctx: &mut RenderContext) {

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
            self.buffer.as_ref().unwrap(),
            ctx.swapchain_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &[
                BufferImageCopy::default()
                    .buffer_image_height(2000)
                    .buffer_row_length(2000)
                    .buffer_offset(0)
                    .image_extent(
                        Extent3D::default()
                            .width(2000)
                            .height(2000)
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

fn main() {
    App::run(
        AppConfig::default(),
         Arc::new(Mutex::new(ComputeRender {
            buffer: None
        })),
        None
    );
}

