use std::sync::{Arc, Mutex};
use cen::graphics::pipeline_store::{PipelineConfig, PipelineKey};
use ash::vk;
use ash::vk::WriteDescriptorSet;
use egui::Context;
use cen::app::App;
use cen::app::app::AppConfig;
use cen::app::gui::{GuiComponent, GuiSystem};
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{CommandBuffer, DescriptorSetLayout, Image};

#[allow(dead_code)]
struct ComputeRender {
    image: Option<Image>,
    descriptorset: Option<DescriptorSetLayout>,
    pipeline_a: Option<PipelineKey>,
    pipeline_b: Option<PipelineKey>,
    pressed: bool,
}

impl RenderComponent for ComputeRender {
    fn initialize(&mut self, renderer: &mut Renderer) {
        // Image
        let image = Image::new(
            &renderer.device,
            &mut renderer.allocator,
            renderer.swapchain.get_extent().width,
            renderer.swapchain.get_extent().height,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST
        );

        // Transition image
        let mut image_command_buffer = CommandBuffer::new(&renderer.device, &renderer.command_pool);
        image_command_buffer.begin();
        {
            renderer.transition_image(&image_command_buffer, image.handle(), vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty(), vk::AccessFlags::empty());
        }
        image_command_buffer.end();
        renderer.submit_single_time_command_buffer(image_command_buffer, Box::new(|| {}));

        // Layout
        let layout_bindings = &[
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE ),
        ];
        let descriptorset = DescriptorSetLayout::new_push_descriptor(
            &renderer.device,
            layout_bindings
        );

        // Pipeline
        let pipeline_a = renderer.pipeline_store().insert(PipelineConfig {
            shader_path: "examples/egui/shader_a.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            push_constant_ranges: vec![],
            macros: Default::default(),
        }).expect("Failed to create pipeline");
        
        let pipeline_b = renderer.pipeline_store().insert(PipelineConfig {
            shader_path: "examples/egui/shader_b.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            push_constant_ranges: vec![],
            macros: Default::default(),
        }).expect("Failed to create pipeline");

        self.image = Some(image);
        self.descriptorset = Some(descriptorset);
        self.pipeline_a = Some(pipeline_a);
        self.pipeline_b = Some(pipeline_b);
    }

    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, swapchain_image: &vk::Image, _: &vk::ImageView) {
        
        // Render
        let compute = if !self.pressed {
            renderer.pipeline_store().get(self.pipeline_a.unwrap()).unwrap()
        } else {
            renderer.pipeline_store().get(self.pipeline_b.unwrap()).unwrap()
        };
        
        command_buffer.bind_pipeline(&compute);

        let bindings = [self.image.as_ref().unwrap().binding(vk::ImageLayout::GENERAL)];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        command_buffer.bind_push_descriptor(
            &compute,
            0,
            &[write_descriptor_set]
        );
        
        command_buffer.dispatch(500, 500, 1 );

        // Transition the render to a source
        renderer.transition_image(
            &command_buffer,
            &self.image.as_ref().unwrap().handle(),
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::SHADER_WRITE,
            vk::AccessFlags::TRANSFER_READ
        );

        // Transition the swapchain image
        renderer.transition_image(
            &command_buffer,
            &swapchain_image,
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
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0]
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }]
            );

            // Use a blit, as a copy doesn't synchronize properly to the swapchain on MoltenVK
            renderer.device.handle().cmd_blit_image(
                command_buffer.handle(),
                *self.image.as_ref().unwrap().handle(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                *swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit::default()
                    .src_offsets([
                        vk::Offset3D::default(),
                        vk::Offset3D::default().x(self.image.as_ref().unwrap().width as i32).y(self.image.as_ref().unwrap().height as i32).z(1)
                    ])
                    .dst_offsets([
                        vk::Offset3D::default(),
                        vk::Offset3D::default().x(self.image.as_ref().unwrap().width as i32).y(self.image.as_ref().unwrap().height as i32).z(1)
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
        }

        // Transfer back to default states
        renderer.transition_image(
            &command_buffer,
            &swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::NONE
        );

        // Transition the render image back
        renderer.transition_image(
            &command_buffer,
            &self.image.as_ref().unwrap().handle(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageLayout::GENERAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::NONE
        );

    }
}

impl GuiComponent for ComputeRender {
    fn gui(&mut self, _: &GuiSystem, ctx: &Context) {
        egui::Window::new("Shader controls")
            .resizable(true)
            .title_bar(true)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.pressed, "Alt");
            }
        );
    }
}

fn main() {
    
    let compute = Arc::new(Mutex::new(ComputeRender {
        image: None,
        descriptorset: None,
        pipeline_a: None,
        pipeline_b: None,
        pressed: false,
    }));
    App::run(
        AppConfig::default(), 
        compute.clone(),
        Some(compute)
    );
}