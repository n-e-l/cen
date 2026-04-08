use std::any::Any;
use std::sync::{Arc, Mutex, Weak};
use ash::vk;
use ash::vk::{DescriptorImageInfo, Extent2D, ImageLayout, ImageView, Sampler};
use crate::vulkan::{Image, ImageConfig, OwnedImage};
use crate::vulkan::Device;
use crate::vulkan::Allocator;
use crate::vulkan::memory::GpuResource;

struct InnerDynamicImage {
    image: OwnedImage
}

impl InnerDynamicImage {
    pub fn resize(&mut self, device: &Device, allocator: &mut Allocator, width: u32, height: u32) {
        self.image = OwnedImage::new(
            device,
            allocator,
            self.image.config()
                .width(width)
                .height(height)
        )
    }
}


/*
 * A wrapper around an image which may be resized.
 */
#[derive(Clone)]
pub struct DynamicImage {
    inner: Arc<Mutex<InnerDynamicImage>>
}

impl DynamicImage {
    pub fn new(device: &Device, allocator: &mut Allocator, config: ImageConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerDynamicImage {
                image: OwnedImage::new(device, allocator, config)
            }))
        }
    }

    pub fn resize(&mut self, device: &Device, allocator: &mut Allocator, width: u32, height: u32) {
        self.inner.lock().unwrap().resize(device, allocator, width, height);
    }

    pub fn weak(&self) -> WeakDynamicImage {
        WeakDynamicImage {
            inner: Arc::downgrade(&self.inner)
        }
    }
}

#[derive(Clone)]
pub struct WeakDynamicImage {
    inner: Weak<Mutex<InnerDynamicImage>>
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
        self.inner.lock().unwrap().image.reference()
    }
}

impl Image for DynamicImage {
    fn handle(&self) -> ash::vk::Image {
        self.inner.lock().unwrap().image.handle()
    }

    fn image_view(&self) -> ImageView {
        self.inner.lock().unwrap().image.image_view()
    }

    fn sampler(&self) -> Sampler {
        self.inner.lock().unwrap().image.sampler()
    }

    fn width(&self) -> u32 {
        self.inner.lock().unwrap().image.width()
    }

    fn height(&self) -> u32 {
        self.inner.lock().unwrap().image.height()
    }

    fn extent(&self) -> Extent2D {
        self.inner.lock().unwrap().image.extent()
    }

    fn binding(&self, layout: ImageLayout) -> DescriptorImageInfo {
        let lock = self.inner.lock().unwrap();
        vk::DescriptorImageInfo::default()
            .image_layout(layout)
            .image_view(lock.image.image_view())
            .sampler(lock.image.sampler())
    }
}