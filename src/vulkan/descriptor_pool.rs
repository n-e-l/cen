use std::sync::Arc;
use ash::vk;
use log::trace;
use crate::vulkan::{Device, LOG_TARGET};
use crate::vulkan::device::DeviceInner;

pub struct DescriptorPool {
    pub device_dep: Arc<DeviceInner>,
    pub descriptor_pool: vk::DescriptorPool,
}

impl DescriptorPool {

    pub fn new(device: &Device, max_sets: u32) -> DescriptorPool {

        let sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: max_sets,
        }];
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&sizes)
            .max_sets(max_sets)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);
        let descriptor_pool = unsafe { device.handle().create_descriptor_pool(&create_info, None).unwrap() };

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