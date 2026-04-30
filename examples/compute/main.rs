use cen::graphics::pipeline_store::PipelineKey;
use ash::vk;
use ash::vk::{Extent3D, WriteDescriptorSet};
use egui::Context;
use winit::event::WindowEvent;
use cen::app::app::{AppComponent, AppConfig, Cen};
use cen::app::engine::CenContext;
use cen::app::gui::{GuiComponent, GuiContext};
use cen::app::{ImageFlags, ImageResource};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{DescriptorSetLayout, ImageTrait, ImageConfig, ComputePipelineConfig};

struct ComputeExample {
    image: ImageResource,
    _descriptorset: DescriptorSetLayout,
    pipeline: PipelineKey,
}

impl AppComponent for ComputeExample {
    fn new(ctx: &mut CenContext) -> Self {
        let image = ctx.create_image(
            ImageConfig {
                extent: Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1
                },
                image_usage_flags: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
                ..Default::default()
            },
            ImageFlags::MATCH_SWAPCHAIN_EXTENT
        );

        let layout_bindings = &[
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        let descriptorset = DescriptorSetLayout::new_push_descriptor(
            &ctx.gfx.device,
            layout_bindings
        );

        let pipeline = ctx.create_pipeline(ComputePipelineConfig {
            shader_source: "examples/compute/shader.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            ..Default::default()
        }).expect("Failed to create pipeline");

        Self {
            image,
            pipeline,
            _descriptorset: descriptorset
        }
    }

    fn window_event(&mut self, _: WindowEvent) {}
}

impl RenderComponent for ComputeExample {
    fn render(&mut self, ctx: &mut CenContext) {
        let image = ctx.images.get(&self.image);

        ctx.command_buffer.transition(image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);

        let compute = ctx.pipelines.get(self.pipeline).unwrap();
        ctx.command_buffer.bind_pipeline(compute);

        let bindings = [image.binding(vk::ImageLayout::GENERAL)];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        ctx.command_buffer.bind_push_descriptor(
            compute,
            0,
            &[write_descriptor_set]
        );
        ctx.command_buffer.dispatch(500, 500, 1);

        ctx.command_buffer.transition(image, vk::ImageLayout::GENERAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);

        let swapchain_image = ctx.swapchain_image.unwrap();

        ctx.command_buffer.transition(swapchain_image, vk::ImageLayout::PRESENT_SRC_KHR, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        ctx.command_buffer.clear_color_image(
            swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            [0.0, 0.0, 0.0, 1.0]
        );

        ctx.command_buffer.blit_image(
            image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::ImageBlit::default()
                .src_offsets([
                    vk::Offset3D::default(),
                    vk::Offset3D::default().x(image.width() as i32).y(image.height() as i32).z(1)
                ])
                .dst_offsets([
                    vk::Offset3D::default(),
                    vk::Offset3D::default().x(image.width() as i32).y(image.height() as i32).z(1)
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

        ctx.command_buffer.transition(swapchain_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR);
        ctx.command_buffer.transition(image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::ImageLayout::GENERAL);
    }
}

impl GuiComponent for ComputeExample {
    fn gui(&mut self, _: &mut GuiContext, _: &Context) {}
}

fn main() {
    Cen::<ComputeExample>::run(AppConfig::default());
}
