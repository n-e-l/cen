use std::sync::{Arc, Mutex};
use gpu_allocator::vulkan::AllocatorCreateDesc;
use log::trace;
use crate::vulkan::device::DeviceInner;
use crate::vulkan::{Device, LOG_TARGET};

pub struct AllocatorInner {
    // IMPORTANT: Ordering matters a lot here. We want to drop the allocator before the device
    pub allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    #[allow(dead_code)]
    pub device_dep: Arc<DeviceInner>,
}

impl Drop for AllocatorInner {
    fn drop(&mut self) {
        let allocator = self.allocator.lock().unwrap();
        let report = allocator.generate_report();
        println!("{:?}", report);
        trace!(target: LOG_TARGET, "Destroyed allocator");
    }
}

pub struct Allocator {
    pub(crate) inner: Arc<Mutex<AllocatorInner>>,
}

impl Allocator {
    pub fn new(device: &Device, desc: &AllocatorCreateDesc) -> Self {
        let allocator = Arc::new( Mutex::new(AllocatorInner {
            device_dep: device.inner.clone(),
            allocator: Arc::new(Mutex::new(gpu_allocator::vulkan::Allocator::new(desc).expect("Failed to create allocator")))
        } ) );

        trace!(target: LOG_TARGET, "Created allocator");

        Self {
            inner: allocator,
        }
    }

    pub fn handle(&self) -> Arc<Mutex<gpu_allocator::vulkan::Allocator>> {
        self.inner.lock().unwrap().allocator.clone()
    }
}