use std::sync::{Arc, Mutex, MutexGuard};
use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationScheme};
use log::{trace};
use crate::vulkan::{Allocator, Device, GpuHandle, LOG_TARGET};
use crate::vulkan::allocator::AllocatorInner;
use crate::vulkan::device::DeviceInner;

pub struct BufferInner {
    pub device_dep: Arc<DeviceInner>,
    pub allocator_dep: Arc<Mutex<AllocatorInner>>,
    pub(crate) buffer: vk::Buffer,
    pub size: vk::DeviceSize,
    pub allocation: Mutex<Option<Allocation>>,
}

pub struct Buffer {
    inner: Arc<BufferInner>,
}

impl Drop for BufferInner {
    fn drop(&mut self) {
        unsafe {
            let buffer_addr = format!("{:?}", self.buffer);
            if let Some(allocation) = self.allocation.lock().unwrap().take() {
                let memory_addr = format!("{:?}, {:?}", allocation.memory(), allocation.chunk_id());
                self.allocator_dep.lock().unwrap().allocator.lock().unwrap().free(allocation).unwrap();
                trace!(target: LOG_TARGET, "Destroyed buffer memory: [{}]", memory_addr)
            }
            self.device_dep.device.destroy_buffer(self.buffer, None);
            trace!(target: LOG_TARGET, "Destroyed buffer: [{}]", buffer_addr)
        }
    }
}

impl GpuHandle for BufferInner {}

impl Buffer {
    pub(crate) fn reference(&self) -> Arc<dyn GpuHandle> {
        self.inner.clone()
    }

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
        let allocation = allocator.handle().lock().unwrap()
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
            inner: Arc::new(BufferInner {
                buffer,
                size,
                allocation: Mutex::new(Some(allocation)),
                device_dep: device.inner.clone(),
                allocator_dep: allocator.inner.clone(),
            })
        }
    }

    pub fn mapped(&self) -> Result<MappedBufferGuard<'_>, BufferError> {

        let allocation_guard = self.inner.allocation.lock().unwrap();

        if let Some(ref allocation) = *allocation_guard {
            if allocation.mapped_ptr().is_some() {
                return Ok(MappedBufferGuard {
                    _guard: allocation_guard,
                });
            } else {
                return Err(BufferError::NotMapped);
            }
        }
        
        Err(BufferError::NotAllocated)
    }

    pub fn binding(&self) -> vk::DescriptorBufferInfo {
       vk::DescriptorBufferInfo::default()
           .buffer(self.inner.buffer)
           .offset(0)
           .range(self.inner.size)
    }

    pub fn handle(&self) -> &vk::Buffer {
        &self.inner.buffer
    }

    pub fn size(&self) -> vk::DeviceSize {
        self.inner.size
    }
}

pub struct MappedBufferGuard<'a> {
    _guard: MutexGuard<'a, Option<Allocation>>,
}

impl<'a> MappedBufferGuard<'a> {
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self._guard.as_mut().unwrap().mapped_slice_mut().expect("Failed to map memory")
    }

    pub fn as_slice(&self) -> &[u8] {
        self._guard.as_ref().unwrap().mapped_slice().expect("Failed to map memory")
    }
}

#[derive(Debug)]
pub enum BufferError {
    NotMapped,
    NotAllocated,
}
