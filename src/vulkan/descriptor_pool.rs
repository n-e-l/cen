use std::sync::Arc;
use ash::vk;
use egui_ash_renderer::vulkan::create_vulkan_descriptor_pool;
use log::trace;
use crate::vulkan::{Device, LOG_TARGET};
use crate::vulkan::device::DeviceInner;

pub struct DescriptorPool {
    pub device_dep: Arc<DeviceInner>,
    pub descriptor_pool: vk::DescriptorPool,
}

impl DescriptorPool {

    pub fn new(device: &Device, max_sets: u32) -> DescriptorPool {

        let descriptor_pool = create_vulkan_descriptor_pool(device.handle(), max_sets).unwrap();

        trace!(target: LOG_TARGET, "Created descriptor pool: {:?}", descriptor_pool);

        Self {
            device_dep: device.inner.clone(),
            descriptor_pool
        }
    }

    pub fn handle(&self) -> vk::DescriptorPool {
        self.descriptor_pool
    }

}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            let command_pool_addr = format!("{:?}", self.descriptor_pool);
            self.device_dep.device.destroy_descriptor_pool(self.descriptor_pool, None);
            trace!(target: LOG_TARGET, "Destroyed command pool: [{}]", command_pool_addr);
        }
    }
}