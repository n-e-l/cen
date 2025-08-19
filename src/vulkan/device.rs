use std::sync::Arc;
use ash::khr::swapchain;
use ash::vk;
use ash::vk::{PipelineStageFlags, Queue};
use log::trace;
use crate::vulkan::{CommandBuffer, Instance, LOG_TARGET};
use crate::vulkan::instance::InstanceInner;

/// A connection to a physical GPU.
pub struct DeviceInner {
    pub instance_dep: Arc<InstanceInner>,
    pub device: ash::Device,
    pub device_push_descriptor: ash::khr::push_descriptor::Device,
    pub queue_family_index: u32,
    pub dynamic_rendering_loader: ash::khr::dynamic_rendering::Device
}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe {
            let device_addr = format!("{:?}", self.device.handle());
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
            trace!(target: LOG_TARGET, "Destroyed device: [{}]", device_addr);
        }
    }
}

pub struct Device {
    pub inner: Arc<DeviceInner>,
}

impl Device {
    pub fn new(instance: &Instance, physical_device: vk::PhysicalDevice, queue_family_index: u32) -> Device {
        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities);

        let device_extension_names_raw = [
            swapchain::NAME.as_ptr(),
            // Push descriptors
            ash::khr::push_descriptor::NAME.as_ptr(),
            // Dynamic rendering
            ash::khr::dynamic_rendering::NAME.as_ptr(),
            // MoltenVK
            #[cfg(target_os = "macos")]
                ash::khr::portability_subset::NAME.as_ptr(),
        ];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };

        let mut dynamic_rendering_features = vk::PhysicalDeviceDynamicRenderingFeatures::default()
            .dynamic_rendering(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features)
            .push_next(&mut dynamic_rendering_features);

        let device = unsafe {
            instance.handle()
                .create_device(physical_device, &device_create_info, None)
        }.unwrap();

        trace!(target: LOG_TARGET, "Created device: {:?}", device.handle());

        let device_push_descriptor = ash::khr::push_descriptor::Device::new(instance.handle(), &device);
        
        let dynamic_rendering_loader = ash::khr::dynamic_rendering::Device::new(instance.handle(), &device);

        let device_inner = DeviceInner {
            instance_dep: instance.inner.clone(),
            device,
            device_push_descriptor,
            queue_family_index,
            dynamic_rendering_loader,
        };

        Self {
            inner: Arc::new(device_inner),
        }
    }

    pub fn handle(&self) -> &ash::Device {
        &self.inner.device
    }

    pub fn get_queue(&self, queue_index: u32) -> Queue {
        unsafe { self.handle().get_device_queue(self.inner.queue_family_index, queue_index) }
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.handle().device_wait_idle().unwrap();
        }
    }

    pub fn wait_for_fence(&self, fence: vk::Fence) {
        unsafe {
            let fences = [fence];
            self.handle()
                .wait_for_fences(&fences, true, u64::MAX)
                .expect("Failed to destroy fence");
        }
    }

    pub fn get_fence_status(&self, fence: vk::Fence) -> bool {
        unsafe {
            self.handle()
                .get_fence_status(fence)
                .expect("Failed to destroy fence")
        }
    }

    pub fn reset_fence(&self, fence: vk::Fence) {
        unsafe {
            let fences = [fence];
            self.handle()
                .reset_fences(&fences)
                .unwrap()
        }
    }

    pub fn submit_single_time_command(
        &self,
        queue: Queue,
        command_buffer: &CommandBuffer
    ) {
        unsafe {
            let command_buffers = [command_buffer.handle()];
            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers);

            let submits = [submit_info];
            self.handle().queue_submit(queue, &submits, command_buffer.fence()).unwrap();
        }
    }

    /// Submit a command buffer for execution
    ///
    /// - `wait_semaphore` - A semaphore to wait on before execution.
    /// - `signal_semaphore` - A semaphore to signal after execution.
    /// - `fence` - A fence to signal once the commandbuffer has finished execution.
    ///
    /// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkQueueSubmit.html
    pub fn submit_command_buffer(
        &self,
        queue: &Queue,
        wait_semaphore: vk::Semaphore,
        signal_semaphore: vk::Semaphore,
        command_buffer: &CommandBuffer
    ) {
        let command_buffers = [command_buffer.handle()];
        let wait_semaphores = [wait_semaphore];
        let signal_semaphores = [signal_semaphore];
        let wait_dst_stage_masks = [PipelineStageFlags::TRANSFER];

        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers)
            .wait_semaphores(&wait_semaphores)
            .signal_semaphores(&signal_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_masks);

        let submits = [submit_info];
        let fence = command_buffer.fence();
        unsafe { self.handle().queue_submit(*queue, &submits, fence).unwrap(); }
    }

    pub fn clone(&self) -> Device {
        Device {
            inner: self.inner.clone(),
        }
    }
}
