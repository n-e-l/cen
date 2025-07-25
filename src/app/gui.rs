use ash::vk;
use ash::vk::{AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearValue, DescriptorPool, DescriptorSet, Image, ImageLayout, ImageView, Offset2D, Rect2D, RenderingAttachmentInfo};
use egui::{Context, FullOutput, ViewportId};
use egui_ash_renderer::{DynamicRendering, Options};
use egui_ash_renderer::vulkan::{create_vulkan_descriptor_pool, create_vulkan_descriptor_set, create_vulkan_descriptor_set_layout};
use egui_winit::State;
use crate::app::Window;
use crate::graphics::Renderer;
use crate::graphics::renderer::RenderComponent;
use crate::vulkan::{CommandBuffer, Device};

pub trait GuiComponent {
    fn gui(&mut self, gui: &GuiSystem, context: &Context);
}

pub struct GuiSystem {
    pub egui_ctx: Context,
    pub egui_winit: State,
    pub egui_renderer: Option<egui_ash_renderer::Renderer>,
    device: Option<Device>,
    egui_output: Option<FullOutput>,
    renderer_descriptor_pool: Option<DescriptorPool>,
}

impl GuiSystem {

    pub fn new(window: &Window) -> Self {
        
        let egui_ctx = Context::default();
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            &window.display_handle(),
            None,
            None,
            None
        );
        
        Self {
            egui_ctx,
            egui_winit,
            egui_renderer: None,
            egui_output: None,
            device: None,
            renderer_descriptor_pool: None,
        }
    }

    pub fn create_texture(&self, image_view: vk::ImageView, sampler: vk::Sampler) -> DescriptorSet {
        let device = self.device.as_ref().unwrap().handle();
        let layout = create_vulkan_descriptor_set_layout(device).unwrap();
        create_vulkan_descriptor_set(
            device,
            layout,
            self.renderer_descriptor_pool.unwrap(),
            image_view,
            sampler,
        ).unwrap()
    }

    pub fn on_window_event(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) {
        let _ = self.egui_winit.on_window_event(window, event);
    }
    
    pub fn update(&mut self, window: &winit::window::Window, components: &mut [&mut dyn GuiComponent]) {

        // Renew gui
        let raw_input = self.egui_winit.take_egui_input(window);
        self.egui_output = Some(self.egui_ctx.run(raw_input, |ctx| {
            for component in &mut *components {
                component.gui(&self, ctx);
            }
        }));
    }
}

impl RenderComponent for GuiSystem {

    fn initialize(&mut self, renderer: &mut Renderer) {
        
        self.device = Some(renderer.device.clone());
        self.renderer_descriptor_pool = Some(create_vulkan_descriptor_pool(renderer.device.handle(), 10000).unwrap());

        #[cfg(any(target_os = "linux", target_os = "windows"))]
        let preferred_format = vk::Format::R8G8B8A8_SRGB;

        #[cfg(target_os = "macos")]
        let preferred_format = vk::Format::B8G8R8A8_SRGB;

        self.egui_renderer = Some(egui_ash_renderer::Renderer::with_gpu_allocator(
            renderer.allocator.inner.lock().unwrap().allocator.clone(),
            renderer.device.handle().clone(),
            DynamicRendering {
                color_attachment_format: preferred_format,
                depth_attachment_format: None,
            },
            Options {
                in_flight_frames: renderer.swapchain.get_image_count() as usize,
                enable_depth_test: false,
                enable_depth_write: false,
                srgb_framebuffer: true
            }
        ).unwrap());

    }

    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, _: &Image, swapchain_image_view: &ImageView) {

        if let Some(output) = self.egui_output.take() {

            // Free textures
            self.egui_renderer.as_mut().unwrap()
                .free_textures(output.textures_delta.free.as_slice()).unwrap();

            // Set textures
            // https://docs.rs/egui-ash-renderer/0.7.0/egui_ash_renderer/#managed-textures
            self.egui_renderer.as_mut().unwrap().set_textures(
                renderer.queue, renderer.command_pool.command_pool, output.textures_delta.set.as_slice()
            ).unwrap();

            let clipped_primitives = self.egui_ctx.tessellate(
                output.shapes,
                output.pixels_per_point
            );

            let color_attachments = vec![
                RenderingAttachmentInfo::default()
                    .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(AttachmentLoadOp::LOAD)
                    .store_op(AttachmentStoreOp::STORE)
                    .clear_value(ClearValue { color: ClearColorValue { float32: [1f32, 0f32, 1f32, 1f32] } })
                    .image_view(*swapchain_image_view)
            ];
            let rendering_info = vk::RenderingInfoKHR::default()
                .render_area(Rect2D { offset: Offset2D { x: 0, y: 0 }, extent: renderer.swapchain.get_extent() })
                .layer_count(1)
                .view_mask(0)
                .color_attachments(&color_attachments);
            command_buffer.begin_rendering(&rendering_info);

            // Egui draw call
            self.egui_renderer.as_mut().unwrap().cmd_draw(
                command_buffer.handle(),
                renderer.swapchain.get_extent(),
                output.pixels_per_point,
                clipped_primitives.as_slice()
            ).unwrap();

            command_buffer.end_rendering();
        }
    }
}