use log::{info};
use std::time::Instant;
use ash::vk;
use ash::vk::{ImageLayout, PhysicalDevice};
use gpu_allocator::vulkan::{AllocatorCreateDesc};
use winit::event_loop::EventLoopProxy;
use crate::app::app::UserEvent;
use crate::app::engine::{CenContext};
use crate::app::ImageFlags;
use crate::app::gui::{GuiData, GuiSystem};
use crate::graphics::context::{GraphicsContext, ImageContext, PipelineContext};
use crate::graphics::image_store::ImageStore;
use crate::graphics::pipeline_store::PipelineStore;
use crate::vulkan::{Allocator, CommandBuffer, CommandPool, Device, Image, Instance, Surface, Swapchain, WindowState};

// -- Traits --

pub trait RenderComponent {
    fn render(&mut self, ctx: &mut CenContext);
}

// -- Renderer --

pub struct Renderer {
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub command_buffers: Vec<CommandBuffer>,
    pub swapchain: Swapchain,
    pub entry: ash::Entry,
    pub surface: Surface,
    pub frame_index: usize,
    pub graphics_context: GraphicsContext,
    pub image_context: ImageContext,
    pub pipeline_context: PipelineContext,
    pub physical_device: PhysicalDevice,
    pub instance: Instance,
    pub start_time: Instant,
    present_mode: vk::PresentModeKHR,
}

impl Renderer {
    pub fn new(window: &WindowState, proxy: EventLoopProxy<UserEvent>, vsync: bool) -> Renderer {
        let entry = ash::Entry::linked();
        let instance = Instance::new(&entry, window);
        let surface = Surface::new(&entry, &instance, window);
        let (physical_device, queue_family_index) = instance.create_physical_device(&entry, &surface);
        let device = Device::new(&instance, physical_device, queue_family_index);
        let queue = device.get_queue(0);
        let command_pool = CommandPool::new(&device, queue_family_index);

        let allocator = Allocator::new(
            &device,
            &AllocatorCreateDesc {
                instance: instance.handle().clone(),
                device: device.handle().clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,  // Ideally, check the BufferDeviceAddressFeatures struct.
                allocation_sizes: Default::default(),
            }
        );

        let present_mode = if vsync {
            vk::PresentModeKHR::FIFO
        } else {
            vk::PresentModeKHR::IMMEDIATE
        };

        info!("Creating initial swapchain");
        let swapchain = Swapchain::new(&instance, &physical_device, &device, window, &surface, present_mode, None);

        let command_buffers = (0..swapchain.get_image_count()).map(|_| {
            CommandBuffer::new(&device, &command_pool, true)
        }).collect::<Vec<CommandBuffer>>();

        let image_available_semaphores = (0..swapchain.get_image_count()).map(|_| unsafe {
            let semaphore_create_info = vk::SemaphoreCreateInfo::default();
            device.handle().create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create semaphore")
        }).collect::<Vec<vk::Semaphore>>();

        let render_finished_semaphores = (0..swapchain.get_image_count()).map(|_| unsafe {
            let semaphore_create_info = vk::SemaphoreCreateInfo::default();
            device.handle().create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create semaphore")
        }).collect::<Vec<vk::Semaphore>>();

        let start_time = std::time::Instant::now();

        let pipeline_store = PipelineStore::new( &device, proxy );
        let pipeline_context = PipelineContext {
            pipeline_store
        };

        let image_store = ImageStore::new();
        let image_context = ImageContext {
            image_store,
            images: Vec::new(),
        };

        let graphics_context = GraphicsContext {
            device,
            allocator,
            queue,
            command_pool,
        };

        Self {
            entry,
            graphics_context,
            image_context,
            pipeline_context,
            physical_device,
            instance,
            surface,
            swapchain,
            render_finished_semaphores,
            image_available_semaphores,
            command_buffers,
            frame_index: 0,
            start_time,
            present_mode,
        }
    }

    pub(crate) fn on_window_recreation(&mut self, gui_data: &mut GuiData, window_state: WindowState) {

        self.graphics_context.device.wait_idle();
        info!("Recreating swapchain");
        self.swapchain = Swapchain::new(&self.instance, &self.physical_device, &self.graphics_context.device, &window_state, &self.surface, self.present_mode, Some(self.swapchain.handle()));

        let resizeable: Vec<_> = self.image_context.images
            .iter()
            .filter_map(|(resource, flags)| {
                if flags.contains(ImageFlags::MATCH_SWAPCHAIN_EXTENT) {
                    resource.upgrade()
                } else {
                    None
                }
            })
            .collect();

        for resource in resizeable {
            let image = self.image_context.image_store.get(&resource.image_key());
            let mut config = image.config();
            config.extent.width = self.swapchain.get_extent().width;
            config.extent.height = self.swapchain.get_extent().height;

            let image_key = self.image_context.image_store.insert(
                Image::new(&self.graphics_context.device, &mut self.graphics_context.allocator, config)
            );

            resource.set_image_key(image_key.clone());
            if resource.texture_key().is_some() {
                let texture = gui_data.create_texture(&mut self.image_context.image_store, image_key).unwrap();
                resource.set_texture_key(texture);
            }
        }
    }

    fn record_command_buffer<'a>(&mut self, gui: &mut GuiSystem, frame_index: usize, image_index: usize, render_components: &mut [&mut dyn RenderComponent]) {

        let mut command_buffer = self.command_buffers[frame_index].clone();

        command_buffer.begin();

        // Store any used textures in the command buffer lifetime
        gui.take_used_textures().iter().for_each(|tex| {
            command_buffer.track(tex);
        });

        let swapchain_image = &self.swapchain.get_images()[image_index];

        // Clear the swapchain image
        command_buffer.image_barrier(
            swapchain_image,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::empty(),
            vk::AccessFlags::MEMORY_WRITE,
        );
        command_buffer.clear_color_image(swapchain_image, ImageLayout::TRANSFER_DST_OPTIMAL, [0.0, 0.0, 0.0, 1.0]);
        command_buffer.image_barrier(
            swapchain_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::PRESENT_SRC_KHR,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::empty(),
        );

        let mut ctx = CenContext {
            gfx: &mut self.graphics_context,
            images: &mut self.image_context,
            pipelines: &mut self.pipeline_context,
            command_buffer: &mut command_buffer,
            swapchain_image: Some(swapchain_image),
        };

        for rc in render_components.iter_mut() {
            rc.render( &mut ctx );
        }

        ctx = CenContext {
            gfx: &mut self.graphics_context,
            images: &mut self.image_context,
            pipelines: &mut self.pipeline_context,
            command_buffer: &mut command_buffer,
            swapchain_image: Some(swapchain_image),
        };
        gui.render( &mut ctx );

        command_buffer.end();
    }

    pub fn draw_frame<'a>(&mut self, gui: &mut GuiSystem, render_components: &mut [&mut dyn RenderComponent]) {

        // Clean up the stores
        self.image_context.cleanup();

        // Wait for the current frame's command buffer to finish executing.
        let fence = self.command_buffers[self.frame_index].fence();
        self.graphics_context.device.wait_for_fence(fence);

        // Acquire image and signal the semaphore
        let image_index = self.swapchain.acquire_next_image(self.image_available_semaphores[self.frame_index]) as usize;

        self.record_command_buffer(gui, self.frame_index, image_index, render_components);

        self.graphics_context.device.reset_fence(fence);
        self.graphics_context.device.submit_command_buffer(
            &self.graphics_context.queue,
            self.image_available_semaphores[self.frame_index],
            self.render_finished_semaphores[image_index],
            &self.command_buffers[self.frame_index]
        );

        self.swapchain.queue_present(
            self.graphics_context.queue,
            self.render_finished_semaphores[image_index],
            image_index as u32
        );

        self.frame_index = ( self.frame_index + 1 ) % self.swapchain.get_image_views().len();
    }

    pub fn submit_single_time_command_buffer(&mut self, command_buffer: CommandBuffer) {
        self.graphics_context.device.submit_single_time_command(
            self.graphics_context.queue,
            &command_buffer
        );
        self.graphics_context.device.wait_for_fence(command_buffer.fence());
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.graphics_context.device.handle().device_wait_idle().unwrap();
            for semaphore in &self.render_finished_semaphores {
                self.graphics_context.device.handle().destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.image_available_semaphores {
                self.graphics_context.device.handle().destroy_semaphore(*semaphore, None);
            }
        }
    }
}
