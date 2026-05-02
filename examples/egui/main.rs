use cen::graphics::pipeline_store::{PipelineKey};
use ash::vk;
use ash::vk::{Extent3D, WriteDescriptorSet};
use egui::{Context, Slider};
use winit::event::WindowEvent;
use cen::app::app::{AppComponent, AppConfig, Cen};
use cen::app::gui::{GuiComponent, GuiContext};
use cen::app::engine::{CenContext};
use cen::app::{ImageFlags, ImageResource};
use cen::graphics::renderer::{RenderComponent};
use cen::vulkan::{DescriptorSetLayout, ImageTrait, ImageConfig, ComputePipelineConfig};

#[allow(dead_code)]
struct EguiExample {
    image: ImageResource,
    texture: ImageResource,
    descriptorset: DescriptorSetLayout,
    pipeline_a: PipelineKey,
    pipeline_b: PipelineKey,
    slider: u32,
    pressed: bool,
}

impl AppComponent for EguiExample {
    fn new(ctx: &mut CenContext) -> Self {

        // We're using the image store in order to get automatic swapchain resizable images
        let image = ctx.create_image(
            ImageConfig {
                extent: Extent3D {
                    width: 100,
                    height: 100,
                    depth: 1
                },
                image_usage_flags: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
                ..Default::default()
            },
            ImageFlags::MATCH_SWAPCHAIN_EXTENT
        );

        let texture = ctx.create_image(
            ImageConfig {
                extent: Extent3D {
                    width: 100,
                    height: 100,
                    depth: 1
                },
                image_usage_flags: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                ..Default::default()
            },
            ImageFlags::empty()
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
            &ctx.gfx.device,
            layout_bindings
        );

        // Pipeline
        let pipeline_a = ctx.create_pipeline(ComputePipelineConfig {
            shader_source: "examples/egui/shader_a.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            push_constant_ranges: vec![],
            macros: Default::default(),
            slang_modules: vec![]
        }).expect("Failed to create pipeline");

        let pipeline_b = ctx.create_pipeline(ComputePipelineConfig {
            shader_source: "examples/egui/shader_b.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            push_constant_ranges: vec![],
            macros: Default::default(),
            slang_modules: vec![]
        }).expect("Failed to create pipeline");

        Self {
            image,
            texture,
            descriptorset,
            pipeline_a,
            pipeline_b,
            slider: 100,
            pressed: false,
        }
    }

    fn window_event(&mut self, _: WindowEvent) {
    }
}

impl RenderComponent for EguiExample {
    fn render(&mut self, ctx: &mut CenContext) {

        // Clear the texture
        let texture_image = ctx.images.get(&self.texture);
        ctx.command_buffer.transition(texture_image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        ctx.command_buffer.clear_color_image(
            texture_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            [0.0, 1.0, 0.0, 1.0]
        );
        ctx.command_buffer.transition(texture_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let image = ctx.images.get(&self.image);

        ctx.command_buffer.transition(image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);

        // Render
        let compute = if !self.pressed {
            ctx.pipelines.get(self.pipeline_a).unwrap()
        } else {
            ctx.pipelines.get(self.pipeline_b).unwrap()
        };

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

        ctx.command_buffer.dispatch(500, 500, 1 );

        ctx.command_buffer.transition(image, vk::ImageLayout::GENERAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        ctx.command_buffer.transition(ctx.swapchain_image.unwrap(), vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        // Copy to the swapchain
        ctx.command_buffer.clear_color_image(
            ctx.swapchain_image.unwrap(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            [0.0, 0.0, 0.0, 1.0]
        );

        // Use a blit, as a copy doesn't synchronize properly to the swapchain on MoltenVK
        ctx.command_buffer.blit_image(
            image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ctx.swapchain_image.unwrap(),
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

        ctx.command_buffer.transition(ctx.swapchain_image.unwrap(), vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR);
        ctx.command_buffer.transition(image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::ImageLayout::GENERAL);

    }
}

impl GuiComponent for EguiExample {
    fn gui(&mut self, gui: &mut GuiContext, ctx: &Context) {
        egui::Window::new("Shader controls")
            .resizable(true)
            .title_bar(true)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.pressed, "Alt");

                ui.label("Dynamic textures:");
                ui.add(Slider::new(&mut self.slider, 10..=1000));

                let size = gui.images.get(&self.texture).extent();
                if size.width != self.slider  {
                    self.texture = gui.create_image(
                        ImageConfig {
                            extent: Extent3D {
                                width: self.slider,
                                height: self.slider,
                                depth: 1
                            },
                            image_usage_flags: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                            ..Default::default()
                        },
                        ImageFlags::empty()
                    );
                }

                let extent = gui.images.get(&self.texture).extent();

                ui.add(egui::Image::new(egui::load::SizedTexture::new(
                    gui.get_texture(&mut self.texture),
                    egui::vec2(extent.width as f32, extent.height as f32),
                )));
            }
        );
    }
}

fn main() {
    Cen::<EguiExample>::run(
        AppConfig::default()
            .vsync(false)
              .resizable(true)
    );
}
