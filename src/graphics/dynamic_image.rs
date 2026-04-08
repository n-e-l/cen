use std::any::Any;
use std::sync::{Arc, Mutex, Weak};
use ash::vk;
use ash::vk::{DescriptorImageInfo, ImageLayout, ImageView, Sampler};
use crate::vulkan::{Image, ImageConfig, OwnedImage};
use crate::vulkan::Device;
use crate::vulkan::Allocator;
use crate::vulkan::memory::GpuResource;

/*
 * A wrapper around an image which may be resized.
 */
#[derive(Clone)]
pub struct DynamicImage {
    inner: Arc<Mutex<OwnedImage>>
}

impl DynamicImage {
    pub fn new(device: &Device, allocator: &mut Allocator, config: ImageConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(OwnedImage::new(device, allocator, config)))
        }
    }

    pub fn resize(&mut self, device: &Device, allocator: &mut Allocator, width: u32, height: u32) {
        let mut lock = self.inner.lock().unwrap();
        let config = lock.config().width(width).height(height);
        *lock = OwnedImage::new(device, allocator, config);
    }

    pub fn weak(&self) -> WeakDynamicImage {
        WeakDynamicImage {
            inner: Arc::downgrade(&self.inner)
        }
    }
}

#[derive(Clone)]
pub struct WeakDynamicImage {
    inner: Weak<Mutex<OwnedImage>>
}

impl WeakDynamicImage {
    pub fn upgrade(&self) -> Option<DynamicImage> {
        Some(DynamicImage {
            inner: self.inner.upgrade()?
        })
    }
}

impl GpuResource for DynamicImage {
    fn reference(&self) -> Arc<dyn Any> {
        self.inner.lock().unwrap().reference()
    }
}

impl Image for DynamicImage {
    fn handle(&self) -> vk::Image {
        self.inner.lock().unwrap().handle()
    }

    fn image_view(&self) -> ImageView {
        self.inner.lock().unwrap().image_view()
    }

    fn sampler(&self) -> Sampler {
        self.inner.lock().unwrap().sampler()
    }

    fn width(&self) -> u32 {
        self.inner.lock().unwrap().width()
    }

    fn height(&self) -> u32 {
        self.inner.lock().unwrap().height()
    }

    fn binding(&self, layout: ImageLayout) -> DescriptorImageInfo {
        let lock = self.inner.lock().unwrap();
        vk::DescriptorImageInfo::default()
            .image_layout(layout)
            .image_view(lock.image_view())
            .sampler(lock.sampler())
    }
}
