use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::{BufferImageCopy, BufferUsageFlags, Extent3D, ImageLayout, ImageSubresourceLayers};
use gpu_allocator::MemoryLocation;
use cen::app::App;
use cen::app::app::AppConfig;
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, CommandBuffer};

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

    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, swapchain_image: &vk::Image, _: &vk::ImageView) {

        // Transition the swapchain image
        command_buffer.transition_image(
            swapchain_image,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::NONE,
            vk::AccessFlags::TRANSFER_WRITE
        );

        // Copy to the swapchain
        unsafe {
            renderer.device.handle().cmd_clear_color_image(
                command_buffer.handle(),
                *swapchain_image,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                &vk::ClearColorValue {
                    float32: [1.0, 0.0, 0.0, 1.0]
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }]
            );

            renderer.device.handle().cmd_copy_buffer_to_image(
                command_buffer.handle(),
                *self.buffer.as_ref().unwrap().handle(),
                *swapchain_image,
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
            )
        }

        // Transfer back to default states
        command_buffer.transition_image(
            swapchain_image,
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

