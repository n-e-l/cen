use log::info;
use std::time::Instant;
use ash::vk;
use ash::vk::{Extent2D, ImageAspectFlags, PhysicalDevice, Queue};
use gpu_allocator::vulkan::{AllocatorCreateDesc};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::{DisplayHandle, WindowHandle};
use crate::app::app::UserEvent;
use crate::graphics::pipeline_store::PipelineStore;
use crate::vulkan::{Allocator, CommandBuffer, CommandPool, Device, Instance, Surface, Swapchain};

pub trait RenderComponent {
    fn initialize(&mut self, renderer: &mut Renderer);
    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, swapchain_image: &vk::Image, swapchain_image_view: &vk::ImageView);
}

pub struct Renderer {
    pub(crate) pipeline_store: PipelineStore,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub command_buffers: Vec<CommandBuffer>,
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
    cb_callbacks: Vec<(CommandBuffer, Box<dyn FnOnce()>)>
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
        let instance = Instance::new(&entry, &window);
        let surface = Surface::new(&entry, &instance, &window);
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
        let swapchain = Swapchain::new(&instance, &physical_device, &device, &window, &surface, present_mode, None);
        Self::transition_swapchain_images(&device, &command_pool, &queue, &swapchain);

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
            pipeline_store,
            frame_index: 0,
            start_time,
            present_mode,
            cb_callbacks: Default::default()
        }
    }

    pub(crate) fn recreate_window(&mut self, window_state: WindowState) {
        info!("Recreating swapchain");
        self.device.wait_idle();
        self.swapchain = Swapchain::new(&self.instance, &self.physical_device, &self.device, &window_state, &self.surface, self.present_mode, Some(self.swapchain.handle()));
        Self::transition_swapchain_images(&self.device, &self.command_pool, &self.queue, &self.swapchain);
    }

    fn transition_swapchain_images(device: &Device, command_pool: &CommandPool, queue: &Queue, swapchain: &Swapchain) {
        let mut image_command_buffer = CommandBuffer::new(device, command_pool, false);

        image_command_buffer.begin();

        swapchain.get_images().iter().for_each(|image| {
            let image_memory_barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::empty())
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(*image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            unsafe {
                device.handle().cmd_pipeline_barrier(
                    image_command_buffer.handle(),
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[image_memory_barrier]
                )
            }
        });
        image_command_buffer.end();

        device.submit_single_time_command(*queue, &image_command_buffer);
        device.wait_for_fence(image_command_buffer.fence());
    }
    
    fn record_command_buffer(&mut self, frame_index: usize, image_index: usize, render_components: &mut [&mut dyn RenderComponent]) {

        let mut command_buffer = self.command_buffers[frame_index].clone();

        command_buffer.begin();

        let swapchain_image = self.swapchain.get_images()[image_index];
        let swapchain_image_view = self.swapchain.get_image_views()[image_index];
        
        for rc in render_components.iter_mut() {
            rc.render( self, &mut command_buffer, &swapchain_image, &swapchain_image_view );
        }

        command_buffer.end();
    }

    pub fn transition_image(
        &self,
        command_buffer: &CommandBuffer,
        image: &vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_flags: vk::AccessFlags,
        dst_access_flags: vk::AccessFlags,
    ) {
        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_access_mask(src_access_flags)
            .dst_access_mask(dst_access_flags)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(*image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        unsafe {
            self.device.handle().cmd_pipeline_barrier(
                command_buffer.handle(),
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier]
            )
        }
    }

    fn update_callbacks(&mut self) {
        // Get the indices to remove
        let mut remove_indices = Vec::new();
        self.cb_callbacks.iter().enumerate().for_each(|(index, (cb, _callback))| {
            if self.device.get_fence_status(cb.fence()) {
                remove_indices.push(index);
            }
        });

        // Remove and execute callbacks
        for i in remove_indices.iter().rev() {
            let (_, callback) = self.cb_callbacks.swap_remove(*i);

            callback();
        }
    }

    pub fn update(&mut self) {
        self.update_callbacks();
    }

    pub fn draw_frame(&mut self, render_component: &mut [&mut dyn RenderComponent]) {

        // Wait for the current frame's command buffer to finish executing.
        let fence = self.command_buffers[self.frame_index].fence();
        self.device.wait_for_fence(fence);

        let image_index = self.swapchain.acquire_next_image(self.image_available_semaphores[self.frame_index]) as usize;

        self.record_command_buffer(self.frame_index, image_index, render_component);

        // Run any wait signals
        self.update_callbacks();

        self.device.reset_fence(fence);
        self.device.submit_command_buffer(
            &self.queue,
            self.image_available_semaphores[self.frame_index],
            self.render_finished_semaphores[self.frame_index],
            &self.command_buffers[self.frame_index]
        );

        self.swapchain.queue_present(
            self.queue,
            self.render_finished_semaphores[self.frame_index],
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

    pub fn add_command_buffer_callback(&mut self, command_buffer: CommandBuffer, callback: Box<dyn FnOnce()>) {
        self.cb_callbacks.push((command_buffer, callback));
    }

    pub fn submit_single_time_command_buffer(&mut self, command_buffer: CommandBuffer) {
        // Transmit
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
