use std::sync::{Arc, Mutex};
use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationScheme};
use log::trace;
use crate::vulkan::{Allocator, Device, LOG_TARGET};
use crate::vulkan::allocator::AllocatorInner;
use crate::vulkan::device::DeviceInner;

pub struct Buffer {
    pub device_dep: Arc<DeviceInner>,
    pub allocator_dep: Arc<Mutex<AllocatorInner>>,
    pub(crate) buffer: vk::Buffer,
    pub size: vk::DeviceSize,
    pub allocation: Option<Allocation>,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            let buffer_addr = format!("{:?}", self.buffer);
            if let Some(allocation) = self.allocation.take() {
                let memory_addr = format!("{:?}, {:?}", allocation.memory(), allocation.chunk_id());
                self.allocator_dep.lock().unwrap().allocator.free(allocation).unwrap();
                trace!(target: LOG_TARGET, "Destroyed buffer memory: [{}]", memory_addr)
            }
            self.device_dep.device.destroy_buffer(self.buffer, None);
            trace!(target: LOG_TARGET, "Destroyed buffer: [{}]", buffer_addr)
        }
    }
}

impl Buffer {
    pub fn new(device: &Device, allocator: &mut Allocator, location: MemoryLocation, size: vk::DeviceSize, buffer_usage_flags: vk::BufferUsageFlags) -> Buffer {

        // Image
        let create_info = vk::BufferCreateInfo::default()
            .usage(buffer_usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .size(size);

        let buffer = unsafe {
            device.handle().create_buffer(&create_info, None)
                .expect("Failed to create buffer")
        };

        trace!(target: LOG_TARGET, "Created buffer: [{:?}]", buffer);

        // Allocate memory
        let requirements = unsafe { device.handle().get_buffer_memory_requirements(buffer) };
        let allocation = allocator.handle().allocator
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Buffer",
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).unwrap();

        unsafe {
            device.handle().bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
            .expect("Failed to bind buffer memory")
        }

        Buffer {
            buffer,
            size,
            allocation: Some(allocation),
            device_dep: device.inner.clone(),
            allocator_dep: allocator.inner.clone(),
        }
    }

    pub fn mapped(&mut self) -> &mut [u8] {
        self.allocation.as_mut().unwrap().mapped_slice_mut().expect("Failed to map memory")
    }

    pub fn binding(&self) -> vk::DescriptorBufferInfo {
       vk::DescriptorBufferInfo::default()
           .buffer(self.buffer)
           .offset(0)
           .range(self.size)
    }

    pub fn handle(&self) -> &vk::Buffer {
        &self.buffer
    }
}