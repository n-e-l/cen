use std::any::Any;
use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::{ComponentMapping, Extent2D, ImageAspectFlags};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationScheme};
use log::{trace};
use crate::vulkan::{Allocator, Device, LOG_TARGET};
use crate::vulkan::allocator::AllocatorInner;
use crate::vulkan::device::DeviceInner;
use crate::vulkan::memory::GpuResource;

enum ImageOrigin {
    External,
    Created
}

struct ImageInner {
    pub device_dep: Arc<DeviceInner>,
    pub allocator_dep: Option<Arc<Mutex<AllocatorInner>>>,
    pub(crate) image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
    pub width: u32,
    pub height: u32,
    pub allocation: Mutex<Option<Allocation>>,
    origin: ImageOrigin,
}

pub struct Image {
    inner: Arc<ImageInner>,
}


impl Drop for ImageInner {
    fn drop(&mut self) {
        unsafe {
            match self.origin {
                ImageOrigin::External => {
                    let image_addr = format!("{:?}", self.image);
                    self.device_dep.device.destroy_sampler(self.sampler, None);
                    self.device_dep.device.destroy_image_view(self.image_view, None);

                    if let Some(allocation) = self.allocation.lock().unwrap().take() {
                        let memory_addr = format!("{:?}, {:?}", allocation.memory(), allocation.chunk_id());
                        self.allocator_dep.as_ref().expect("").lock().unwrap().allocator.lock().unwrap().free(allocation).unwrap();
                        trace!(target: LOG_TARGET, "Destroyed image memory: [{}]", memory_addr);
                    }

                    // Don't destroy the image, it's external
                    trace!(target: LOG_TARGET, "Destroyed external image data: [{}]", image_addr);
                }
                ImageOrigin::Created => {
                    let image_addr = format!("{:?}", self.image);
                    self.device_dep.device.destroy_sampler(self.sampler, None);
                    self.device_dep.device.destroy_image_view(self.image_view, None);

                    if let Some(allocation) = self.allocation.lock().unwrap().take() {
                        let memory_addr = format!("{:?}, {:?}", allocation.memory(), allocation.chunk_id());
                        self.allocator_dep.as_ref().expect("").lock().unwrap().allocator.lock().unwrap().free(allocation).unwrap();
                        trace!(target: LOG_TARGET, "Destroyed image memory: [{}]", memory_addr);
                    }

                    self.device_dep.device.destroy_image(self.image, None);
                    trace!(target: LOG_TARGET, "Destroyed image: [{}]", image_addr);
                }
            }
        }
    }
}

impl GpuResource for Image {
    fn reference(&self) -> Arc<dyn Any> {
        self.inner.clone()
    }
}

impl Image {

    pub fn from_raw(device: &Device, image: vk::Image, format: vk::Format, extent: Extent2D) -> Image {
        // Image view
        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .format(format)
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .components(ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe {
            device.handle().create_image_view(&image_view_create_info, None)
                .expect("Failed to create image")
        };

        let sampler_create_info = vk::SamplerCreateInfo::default();

        // Sampler
        let sampler = unsafe {
            device.handle().create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler")
        };

        Image {
            inner: Arc::new(ImageInner {
                image,
                image_view,
                sampler,
                allocation: Mutex::new(None),
                device_dep: device.inner.clone(),
                allocator_dep: None,
                width: extent.width,
                height: extent.height,
                origin: ImageOrigin::External,
            })
        }
    }

    pub fn new(device: &Device, allocator: &mut Allocator, width: u32, height: u32, image_usage_flags: vk::ImageUsageFlags) -> Image {

        // Image
        let create_info = vk::ImageCreateInfo::default()
            .extent(vk::Extent3D {
                width: width,
                height: height,
                depth: 1,
            })
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(image_usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .array_layers(1)
            .mip_levels(1)
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM);

        let image = unsafe {
            device.handle().create_image(&create_info, None)
                .expect("Failed to create image")
        };

        trace!(target: LOG_TARGET, "Created image: [{:?}]", image);

        // Allocate memory
        let requirements = unsafe { device.handle().get_image_memory_requirements(image) };
        let allocation = allocator.handle().lock().unwrap()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Image",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).unwrap();

        unsafe {
            device.handle().bind_image_memory(image, allocation.memory(), allocation.offset())
            .expect("Failed to bind image memory")
        }

        // Image view
        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .format(vk::Format::R8G8B8A8_UNORM)
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .components(ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe {
            device.handle().create_image_view(&image_view_create_info, None)
                .expect("Failed to create image")
        };

        let sampler_create_info = vk::SamplerCreateInfo::default();

        // Sampler
        let sampler = unsafe {
            device.handle().create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler")
        };

        Image {
            inner: Arc::new(ImageInner {
                image,
                image_view,
                sampler,
                allocation: Mutex::new(Some(allocation)),
                device_dep: device.inner.clone(),
                allocator_dep: Some(allocator.inner.clone()),
                width,
                height,
                origin: ImageOrigin::Created,
            })
        }
    }

    pub fn binding(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
       vk::DescriptorImageInfo::default()
            .image_layout(layout)
            .image_view(self.inner.image_view)
            .sampler(self.inner.sampler)
    }

    pub fn handle(&self) -> &vk::Image {
        &self.inner.image
    }

    pub fn image_view(&self) -> vk::ImageView {
        self.inner.image_view
    }

    pub fn sampler(&self) -> vk::Sampler {
        self.inner.sampler
    }

    pub fn width(&self) -> u32 {
        self.inner.width
    }

    pub fn height(&self) -> u32 {
        self.inner.height
    }

    pub fn extent(&self) -> Extent2D {
        Extent2D {
            width: self.inner.width,
            height: self.inner.height,
        }
    }
}

