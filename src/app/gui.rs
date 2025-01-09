use std::sync::{Arc, Mutex, Weak};
use ash::vk;
use ash::vk::{AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearValue, Format, Image, ImageLayout, ImageView, Offset2D, Rect2D, RenderingAttachmentInfo};
use egui::{Context, ViewportId};
use egui_ash_renderer::{DynamicRendering, Options};
use egui_winit::State;
use winit::raw_window_handle::{HasDisplayHandle};
use crate::app::Window;
use crate::graphics::Renderer;
use crate::graphics::renderer::RenderComponent;
use crate::vulkan::{CommandBuffer};

pub struct GuiComponent {
    pub egui_ctx: Option<Context>,
    pub egui_winit: Option<State>,
    pub egui_renderer: Option<egui_ash_renderer::Renderer>,
    window: Weak<Mutex<Window>>,
}

impl GuiComponent {

    pub fn new(window: Weak<Mutex<Window>>) -> Self {
        Self {
            egui_ctx: None,
            egui_winit: None,
            egui_renderer: None,
            window,
        }
    }

    pub fn on_window_event(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) {
        let _ = self.egui_winit.as_mut().unwrap().on_window_event(window, event);
    }
}

impl RenderComponent for GuiComponent {

    fn initialize(&mut self, renderer: &mut Renderer) {
        self.egui_renderer = Some(egui_ash_renderer::Renderer::with_gpu_allocator(
            renderer.allocator.inner.lock().unwrap().allocator.clone(),
            renderer.device.handle().clone(),
            DynamicRendering {
                color_attachment_format: Format::B8G8R8A8_UNORM,
                depth_attachment_format: None,
            },
            Options {
                in_flight_frames: renderer.swapchain.get_image_count() as usize,
                enable_depth_test: false,
                enable_depth_write: false,
                srgb_framebuffer: true
            }
        ).unwrap());

        self.egui_ctx = Some(Context::default());
        self.egui_winit = Some(egui_winit::State::new(
            self.egui_ctx.as_mut().unwrap().clone(),
            ViewportId::ROOT,
            &self.window.upgrade().unwrap().lock().unwrap().display_handle(),
            None,
            None,
            None
        ));
    }

    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, _: &Image, swapchain_image_view: &ImageView) {

        // Renew gui
        let raw_input = self.egui_winit.as_mut().unwrap().take_egui_input(&self.window.upgrade().unwrap().lock().unwrap().winit_window());
        let mut egui_output = Some(self.egui_ctx.as_mut().unwrap().run(raw_input, |ctx| {
            egui::Window::new("GUI")
                .resizable(true)
                .title_bar(true)
                .show(ctx, |ui| {
                    if ui.button("test button!").clicked() {
                        
                    }
                }
            );
        }));

        if let Some(output) = egui_output.take() {

            // Set textures
            // https://docs.rs/egui-ash-renderer/0.7.0/egui_ash_renderer/#managed-textures
            self.egui_renderer.as_mut().unwrap().set_textures(
                renderer.queue, renderer.command_pool.command_pool, output.textures_delta.set.as_slice()
            ).unwrap();

            let clipped_primitives = self.egui_ctx.as_mut().unwrap().tessellate(
                output.shapes,
                output.pixels_per_point
            );

            let color_attachments = vec![
                RenderingAttachmentInfo::default()
                    .image_layout(ImageLayout::GENERAL)
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