use std::sync::{Arc, Mutex};
use cen::graphics::pipeline_store::{PipelineConfig, PipelineKey};
use ash::vk;
use ash::vk::WriteDescriptorSet;
use cen::app::Cen;
use cen::app::app::AppConfig;
use cen::app::component::{Component, ComponentRegistry};
use cen::app::engine::InitContext;
use cen::graphics::renderer::{RenderComponent, RenderContext};
use cen::vulkan::{DescriptorSetLayout, Image};

#[allow(dead_code)]
struct ComputeRender {
    image: Image,
    descriptorset: DescriptorSetLayout,
    pipeline: PipelineKey,
}

impl ComputeRender {
    pub fn new(ctx: &mut InitContext) -> Self {

        // Image
        let image = Image::new(
            &ctx.device,
            &mut ctx.allocator,
            ctx.swapchain_extent.width,
            ctx.swapchain_extent.height,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST
        );

        // Layout
        let layout_bindings = &[
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE ),
        ];
        let descriptorset = DescriptorSetLayout::new_push_descriptor(
            &ctx.device,
            layout_bindings
        );

        // Pipeline
        let pipeline = ctx.pipeline_store.insert(PipelineConfig {
            shader_path: "examples/compute/shader.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            push_constant_ranges: vec![],
            macros: Default::default(),
        }).expect("Failed to create pipeline");

        Self {
            image,
            pipeline,
            descriptorset
        }
    }
}

impl RenderComponent for ComputeRender {
    fn render(&mut self, ctx: &mut RenderContext) {

        if self.image.width() != ctx.swapchain_image.width() || self.image.height() != ctx.swapchain_image.height() {
            // Recreate image
            self.image = Image::new(
                &ctx.device,
                &mut ctx.allocator,
                ctx.swapchain_image.width(),
                ctx.swapchain_image.height(),
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST
            );
            ctx.command_buffer.image_barrier(
                &self.image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::AccessFlags::empty(),
                vk::AccessFlags::empty()
            );
        }

        // Render
        let compute = ctx.pipeline_store.get(self.pipeline).unwrap();
        ctx.command_buffer.bind_pipeline(&compute);

        let bindings = [self.image.binding(vk::ImageLayout::GENERAL)];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        ctx.command_buffer.bind_push_descriptor(
            &compute,
            0,
            &[write_descriptor_set]
        );
        ctx.command_buffer.dispatch(500, 500, 1 );

        // Transition the render to a source
        ctx.command_buffer.image_barrier(
            &self.image,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::SHADER_WRITE,
            vk::AccessFlags::TRANSFER_READ
        );

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
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            [0.0, 0.0, 0.0, 1.0]
        );

        // Use a blit, as a copy doesn't synchronize properly to the swapchain on MoltenVK
        ctx.command_buffer.blit_image(
            &self.image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ctx.swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::ImageBlit::default()
                .src_offsets([
                    vk::Offset3D::default(),
                    vk::Offset3D::default().x(self.image.width() as i32).y(self.image.height() as i32).z(1)
                ])
                .dst_offsets([
                    vk::Offset3D::default(),
                    vk::Offset3D::default().x(self.image.width() as i32).y(self.image.height() as i32).z(1)
                ])
                .src_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .mip_level(0)
                )
                .dst_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .mip_level(0)
                )
            ],
            vk::Filter::NEAREST,
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

        // Transition the render image back
        ctx.command_buffer.image_barrier(
            &self.image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageLayout::GENERAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::NONE
        );

    }
}

fn main() {
    Cen::run(
        AppConfig::default(), 
        Box::new(|ctx| {
            let compute = Arc::new(Mutex::new(ComputeRender::new(ctx)));
            ComponentRegistry::new()
                .register(Component::Render(compute))
        })
    );
}
