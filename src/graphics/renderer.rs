use std::sync::{Arc, Mutex};
use log::{info};
use std::time::Instant;
use ash::vk;
use ash::vk::{Extent2D, ImageLayout, PhysicalDevice, Queue};
use gpu_allocator::vulkan::{AllocatorCreateDesc};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::{DisplayHandle, WindowHandle};
use crate::app::app::UserEvent;
use crate::graphics::pipeline_store::PipelineStore;
use crate::vulkan::{Allocator, CommandBuffer, CommandPool, Device, Image, Instance, Surface, Swapchain};

pub struct RenderContext<'a> {
    pub device: &'a Device,
    pub allocator: &'a mut Allocator,
    pub pipeline_store: &'a PipelineStore,
    pub command_buffer: &'a mut CommandBuffer,
    pub swapchain_image: &'a Image,
    pub queue: &'a Queue,
    pub command_pool: &'a CommandPool,
    on_finish: &'a mut Vec<Box<dyn FnOnce()>>
}

impl RenderContext<'_> {
    pub fn run_on_finish(&mut self, fun: Box<dyn FnOnce()>) {
        self.on_finish.push(fun);
    }
}

pub trait RenderComponent {
    fn render(&mut self, ctx: &mut RenderContext);
}

pub struct Renderer {
    pub(crate) pipeline_store: PipelineStore,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub command_buffers: Vec<CommandBuffer>,
    pub on_finish_functions: Vec<Vec<Box<dyn FnOnce()>>>,
    pub command_pool: CommandPool,
    pub queue: Queue,
    pub swapchain: Swapchain,
    pub entry: ash::Entry,
    pub surface: Surface,
    pub frame_index: usize,
    pub allocator: Allocator,
    pub device: Device,
    pub physical_device: PhysicalDevice,
    pub instance: Instance,
    pub start_time: Instant,
    present_mode: vk::PresentModeKHR,
}

pub struct WindowState<'a> {
    pub window_handle: WindowHandle<'a>,
    pub display_handle: DisplayHandle<'a>,
    pub extent2d: Extent2D,
    pub scale_factor: f64,
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

        let on_finish_functions = (0..swapchain.get_image_count()).map(|_| {
            vec![]
        }).collect::<Vec<Vec<Box<dyn FnOnce()>>>>();

        let pipeline_store = PipelineStore::new( &device, proxy );

        let start_time = std::time::Instant::now();

        Self {
            entry,
            device,
            physical_device,
            instance,
            allocator,
            surface,
            queue,
            swapchain,
            render_finished_semaphores,
            image_available_semaphores,
            command_pool,
            command_buffers,
            on_finish_functions,
            pipeline_store,
            frame_index: 0,
            start_time,
            present_mode,
        }
    }

    pub(crate) fn recreate_window(&mut self, window_state: WindowState) {
        info!("Recreating swapchain");
        self.device.wait_idle();
        self.swapchain = Swapchain::new(&self.instance, &self.physical_device, &self.device, &window_state, &self.surface, self.present_mode, Some(self.swapchain.handle()));
    }

    fn record_command_buffer(&mut self, frame_index: usize, image_index: usize, render_components: &[Arc<Mutex<dyn RenderComponent>>]) {

        let mut command_buffer = self.command_buffers[frame_index].clone();

        command_buffer.begin();

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

        let mut ctx = RenderContext {
            device: &self.device,
            allocator: &mut self.allocator,
            pipeline_store: &self.pipeline_store,
            command_buffer: &mut command_buffer,
            swapchain_image,
            queue: &self.queue,
            command_pool: &self.command_pool,
            on_finish: &mut self.on_finish_functions[frame_index]
        };

        for rc in render_components.iter() {
            rc.lock().unwrap().render( &mut ctx );
        }

        command_buffer.end();
    }

    pub fn draw_frame(&mut self, render_components: &[Arc<Mutex<dyn RenderComponent>>]) {

        // Wait for the current frame's command buffer to finish executing.
        let fence = self.command_buffers[self.frame_index].fence();
        self.device.wait_for_fence(fence);

        // Run the finish functions
        for f in self.on_finish_functions[self.frame_index].drain(..) {
            f();
        }

        // Acquire image and signal the semaphore
        let image_index = self.swapchain.acquire_next_image(self.image_available_semaphores[self.frame_index]) as usize;

        self.record_command_buffer(self.frame_index, image_index, render_components);

        self.device.reset_fence(fence);
        self.device.submit_command_buffer(
            &self.queue,
            self.image_available_semaphores[self.frame_index],
            self.render_finished_semaphores[image_index],
            &self.command_buffers[self.frame_index]
        );

        self.swapchain.queue_present(
            self.queue,
            self.render_finished_semaphores[image_index],
            image_index as u32
        );

        self.frame_index = ( self.frame_index + 1 ) % self.swapchain.get_image_views().len();
    }

    pub fn pipeline_store(&mut self) -> &mut PipelineStore {
        &mut self.pipeline_store
    }

    pub fn create_command_buffer(&mut self) -> CommandBuffer {
        CommandBuffer::new(&self.device, &self.command_pool, false)
    }

    pub fn submit_single_time_command_buffer(&mut self, command_buffer: CommandBuffer) {
        self.device.submit_single_time_command(
            self.queue,
            &command_buffer
        );
        self.device.wait_for_fence(command_buffer.fence());
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().device_wait_idle().unwrap();
            for semaphore in &self.render_finished_semaphores {
                self.device.handle().destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.image_available_semaphores {
                self.device.handle().destroy_semaphore(*semaphore, None);
            }
        }
    }
}
