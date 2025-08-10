use ash::vk;
use ash::vk::{AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearValue, DescriptorSet, DescriptorSetLayout, ImageLayout, ImageView, Offset2D, Rect2D, RenderingAttachmentInfo};
use egui::{Context, FullOutput, TextureId, ViewportId};
use egui_ash_renderer::{DynamicRendering, Options};
use egui_ash_renderer::vulkan::{create_vulkan_descriptor_set, create_vulkan_descriptor_set_layout};
use egui_winit::State;
use crate::app::Window;
use crate::graphics::Renderer;
use crate::graphics::renderer::RenderComponent;
use crate::vulkan::{CommandBuffer, Device, DescriptorPool, Image};
use std::collections::HashMap;
use log::{trace};

pub trait GuiComponent {
    fn initialize_gui(&mut self, gui: &mut GuiSystem);
    fn gui(&mut self, gui: &GuiSystem, context: &Context);
}

pub struct GuiSystem {
    pub egui_ctx: Context,
    pub egui_winit: State,
    pub egui_renderer: Option<egui_ash_renderer::Renderer>,
    device: Option<Device>,
    renderer_descriptor_pool: Option<DescriptorPool>,
    egui_output: Option<FullOutput>,
    texture_layout: Option<DescriptorSetLayout>,
    user_textures: HashMap<TextureId, DescriptorSet>,
}

impl Drop for GuiSystem {
    fn drop(&mut self) {
        for (id, _) in self.user_textures.iter() {
            self.egui_renderer.as_mut().unwrap().remove_user_texture(*id);
            trace!("Destroyed user texture {:?}", id);
        }
        if let Some(device) = self.device.as_ref() {
            unsafe {
                device.handle().destroy_descriptor_set_layout(self.texture_layout.unwrap(), None);
                trace!("Destroyed gui image descriptor set layout {:?}", self.texture_layout.unwrap());
            }
        }
    }
}

impl GuiSystem {

    pub fn new(window: &Window) -> Self {
        
        let egui_ctx = Context::default();

        // Enable image loading
        // You will still need to add a loader to your imports. e.g.
        // image = { version = "0.25", features = ["png"] }
        egui_extras::install_image_loaders(&egui_ctx);

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
            texture_layout: None,
            user_textures: HashMap::new(),
        }
    }

    pub fn create_texture(&mut self, image: &Image) -> TextureId {
        let device = self.device.as_ref().unwrap().handle();
        let descriptor_set = create_vulkan_descriptor_set(
            device,
            *self.texture_layout.as_ref().unwrap(),
            self.renderer_descriptor_pool.as_ref().unwrap().handle(),
            image.image_view,
            image.sampler,
        ).unwrap();

        let texture_id = self.egui_renderer.as_mut().unwrap().add_user_texture(descriptor_set);

        self.user_textures.insert(texture_id, descriptor_set);

        texture_id
    }

    pub fn remove_texture(&mut self, texture_id: TextureId) {
        unsafe {
            let set = self.user_textures.remove(&texture_id).unwrap();
            self.device.as_ref().unwrap().handle().free_descriptor_sets(self.renderer_descriptor_pool.as_ref().unwrap().descriptor_pool, &[set]).unwrap();
        }
        self.egui_renderer.as_mut().unwrap().remove_user_texture(texture_id);
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
        self.renderer_descriptor_pool = Some(DescriptorPool::new(&renderer.device, 10000));

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

        self.texture_layout = Some(create_vulkan_descriptor_set_layout(self.device.as_ref().unwrap().handle()).unwrap());
    }

    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, _: &vk::Image, swapchain_image_view: &ImageView) {

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
