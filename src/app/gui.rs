use crate::app::{ImageResource, Window};
use crate::graphics::{GraphicsContext, ImageContext};
use crate::graphics::renderer::RenderComponent;
use crate::graphics::Renderer;
use crate::vulkan::memory::GpuResource;
use crate::vulkan::{DescriptorPool, Device, ImageTrait};
use ash::vk;
use ash::vk::{AccessFlags, AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearValue, DescriptorSet, DescriptorSetLayout, ImageLayout, Offset2D, PipelineStageFlags, Rect2D, RenderingAttachmentInfo};
use egui::{Context, FullOutput, TextureId, ViewportId};
use egui_ash_renderer::vulkan::{create_vulkan_descriptor_set, create_vulkan_descriptor_set_layout};
use egui_ash_renderer::{DynamicRendering, Options};
use egui_winit::State;
use log::{error, trace};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use crate::app::engine::CenContext;
use crate::graphics::image_store::{ImageKey, ImageStore};

#[derive(Clone)]
#[derive(Eq, Hash, PartialEq)]
pub(crate) struct TextureHandle {
    pub(crate) image_key: ImageKey,
    pub(crate) id: TextureId
}

pub type TextureKey = Arc<TextureHandle>;

impl GpuResource for TextureKey {
    fn reference(&self) -> Arc<dyn Any> {
        self.clone()
    }
}

pub struct GuiContext<'a> {
    gui_data: &'a mut GuiData,
    pub gfx: &'a mut GraphicsContext,
    pub images: &'a mut ImageContext,
    used_textures: Vec<TextureKey>
}

impl GuiData {

    pub fn create_texture(&mut self, image_store: &mut ImageStore, image: ImageKey) -> Option<TextureKey> {
        if let Some(si) = image_store.get_handle(&image) {

            let device = self.device.handle();
            let descriptor_set = create_vulkan_descriptor_set(
                device,
                self.texture_layout,
                self.renderer_descriptor_pool.handle(),
                si.image.image_view(),
                si.image.sampler(),
            ).unwrap();

            let handle = TextureHandle {
                image_key: image.clone(),
                id: self.egui_renderer.add_user_texture(descriptor_set)
            };
            let texture: TextureKey  = Arc::new(handle.clone());

            self.textures.insert(handle, (Arc::downgrade(&texture), descriptor_set, si.image.reference()));

            return Some(texture);
        }
        None
    }

    pub fn get_texture(&mut self, image_store: &mut ImageStore, resource: &mut ImageResource) -> TextureKey {
        if let Some(texture_key) = resource.texture_key() {
            texture_key
        } else {
            // Create the texture
            let texture_key = self.create_texture(image_store, resource.image_key()).unwrap();
            resource.set_texture_key(texture_key.clone());
            texture_key
        }
    }
}

impl GuiContext<'_> {
    pub fn create_texture(&mut self, image: ImageKey) -> Option<TextureKey> {
        self.gui_data.create_texture(&mut self.images.image_store, image)
    }

    pub fn get_texture(&mut self, resource: &mut ImageResource) -> TextureId
    {
        let key = self.gui_data.get_texture(&mut self.images.image_store, resource);

        // Share the key with the command buffer
        self.used_textures.push(key.clone());

        key.id
    }
}

pub trait GuiComponent {
    fn gui(&mut self, gui: &mut GuiContext, ctx: &Context);
}

type TextureMap = HashMap<TextureHandle, (Weak<TextureHandle>, DescriptorSet, Arc<dyn Any>)>;

pub struct GuiData {
    device: Device,
    pub textures: TextureMap,
    pub egui_renderer: egui_ash_renderer::Renderer,
    texture_layout: DescriptorSetLayout,
    renderer_descriptor_pool: DescriptorPool,
}


pub struct GuiSystem {
    pub egui_ctx: Context,
    pub egui_winit: State,
    pub gui_data: GuiData,
    used_textures: Vec<TextureKey>,
    egui_output: Option<FullOutput>,
}

impl GuiSystem {
    pub(crate) fn take_used_textures(&mut self) -> Vec<TextureKey> {
        std::mem::take(&mut self.used_textures)
    }
}

impl Drop for GuiSystem {
    fn drop(&mut self) {
        for (handle, (_, set, _)) in self.gui_data.textures.iter() {
            self.gui_data.egui_renderer.remove_user_texture(handle.id);
            unsafe {
                self.gui_data.device.handle().free_descriptor_sets(self.gui_data.renderer_descriptor_pool.descriptor_pool, &[*set]).unwrap();
            }
            trace!("Destroyed user texture {:?}", handle.id);
        }
        unsafe {
            self.gui_data.device.handle().destroy_descriptor_set_layout(self.gui_data.texture_layout, None);
            trace!("Destroyed gui image descriptor set layout {:?}", self.gui_data.texture_layout);
        }
    }
}

impl GuiSystem {

    pub fn new(window: &Window, renderer: &mut Renderer) -> Self {

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

        // Renderer values

        let device = renderer.graphics_context.device.clone();
        let renderer_descriptor_pool = DescriptorPool::new(&renderer.graphics_context.device, 10000);

        let preferred_format = renderer.swapchain.get_format().format;

        let egui_renderer = egui_ash_renderer::Renderer::with_gpu_allocator(
            renderer.graphics_context.allocator.inner.lock().unwrap().allocator.clone(),
            renderer.graphics_context.device.handle().clone(),
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
        ).unwrap();

        let texture_layout = create_vulkan_descriptor_set_layout(renderer.graphics_context.device.handle()).unwrap();

        let gui_data = GuiData {
            device,
            renderer_descriptor_pool,
            textures: HashMap::new(),
            egui_renderer,
            texture_layout
        };

        Self {
            egui_ctx,
            egui_winit,
            egui_output: None,
            gui_data,
            used_textures: vec![],
        }
    }

    pub fn on_window_event(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) {
        let _ = self.egui_winit.on_window_event(window, event);
    }

    pub fn update(&mut self, gfx: &mut GraphicsContext, image_context: &mut ImageContext, window: &winit::window::Window, components: &mut [&mut dyn GuiComponent]) {

        // Remove unused images
        self.gui_data.textures.retain(|handle, (texture, set, _)| {
            match texture.upgrade() {
                None => {
                    // There are no more shared references to the texture, so it may be removed
                    unsafe {
                        trace!("Destroyed texture {:?}", *set);
                        self.gui_data.device.handle().free_descriptor_sets(self.gui_data.renderer_descriptor_pool.descriptor_pool, &[*set]).unwrap();
                    }
                    self.gui_data.egui_renderer.remove_user_texture(handle.id);
                    false
                }
                Some(_) => { true }
            }
        });

        let raw_input = self.egui_winit.take_egui_input(window);

        let mut gui_context = GuiContext {
            gui_data: &mut self.gui_data,
            gfx,
            images: image_context,
            used_textures: vec![]
        };

        self.egui_output = Some(self.egui_ctx.run(raw_input, |ctx| {
            for component in &mut *components {
                component.gui(&mut gui_context, ctx);
            }
        }));

        self.used_textures = gui_context.used_textures;
    }

    pub fn context<'a>(&'a mut self, gfx: &'a mut GraphicsContext, image_context: &'a mut ImageContext) -> GuiContext<'a> {
        GuiContext {
            gui_data: &mut self.gui_data,
            gfx,
            images: image_context,
            used_textures: vec![]
        }
    }
}

impl RenderComponent for GuiSystem {

    fn render(&mut self, ctx: &mut CenContext) {

        // Moved all used textures into the command buffer
        for t in self.used_textures.drain(..) {
            ctx.command_buffer.track(&t);
        }

        // Render the gui
        if let Some(output) = self.egui_output.take() {

            // Free textures
            self.gui_data.egui_renderer
                .free_textures(output.textures_delta.free.as_slice()).unwrap();

            // Set textures
            // https://docs.rs/egui-ash-renderer/0.7.0/egui_ash_renderer/#managed-textures
            self.gui_data.egui_renderer.set_textures(
                ctx.gfx.queue, ctx.gfx.command_pool.command_pool, output.textures_delta.set.as_slice()
            ).unwrap();

            let clipped_primitives = self.egui_ctx.tessellate(
                output.shapes,
                output.pixels_per_point
            );

            // Ensure the swapchain image is in the correct layout
            ctx.command_buffer.image_barrier(
                ctx.swapchain_image.unwrap(),
                ImageLayout::PRESENT_SRC_KHR,
                ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                PipelineStageFlags::TOP_OF_PIPE,
                PipelineStageFlags::FRAGMENT_SHADER,
                AccessFlags::NONE,
                AccessFlags::SHADER_WRITE
            );

            let color_attachments = vec![
                RenderingAttachmentInfo::default()
                    .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(AttachmentLoadOp::LOAD)
                    .store_op(AttachmentStoreOp::STORE)
                    .clear_value(ClearValue { color: ClearColorValue { float32: [1f32, 0f32, 1f32, 1f32] } })
                    .image_view(ctx.swapchain_image.unwrap().image_view())
            ];
            let rendering_info = vk::RenderingInfoKHR::default()
                .render_area(Rect2D { offset: Offset2D { x: 0, y: 0 }, extent: ctx.swapchain_image.unwrap().extent() })
                .layer_count(1)
                .view_mask(0)
                .color_attachments(&color_attachments);
            ctx.command_buffer.begin_rendering(&rendering_info);

            // Egui draw call
            match self.gui_data.egui_renderer.cmd_draw(
                ctx.command_buffer.handle(),
                vk::Extent2D { width: ctx.swapchain_image.unwrap().width(), height: ctx.swapchain_image.unwrap().height() },
                output.pixels_per_point,
                clipped_primitives.as_slice()
            ) {
                Ok(_) => (),
                Err(e) => {
                    error!("{}", e);
                }
            }

            ctx.command_buffer.end_rendering();

            // Set the swapchain image back to present
            ctx.command_buffer.image_barrier(
                ctx.swapchain_image.unwrap(),
                ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ImageLayout::PRESENT_SRC_KHR,
                PipelineStageFlags::FRAGMENT_SHADER,
                PipelineStageFlags::BOTTOM_OF_PIPE,
                AccessFlags::SHADER_WRITE,
                AccessFlags::NONE
            );
        }
    }
}
