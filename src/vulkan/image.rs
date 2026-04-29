use std::any::Any;
use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::{ComponentMapping, DescriptorImageInfo, Extent2D, ImageAspectFlags, ImageLayout, ImageView, Sampler};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationScheme};
use log::{trace};
use crate::vulkan::{Allocator, Device, LOG_TARGET};
use crate::vulkan::allocator::AllocatorInner;
use crate::vulkan::device::DeviceInner;
use crate::vulkan::memory::GpuResource;

#[derive(Copy, Clone)]
pub struct ImageConfig {
    pub extent: vk::Extent3D,
    pub samples: vk::SampleCountFlags,
    pub image_usage_flags: vk::ImageUsageFlags,
    pub image_create_flags: vk::ImageCreateFlags,
    pub image_view_create_flags: vk::ImageViewCreateFlags,
    pub sharing_mode: vk::SharingMode,
    pub initial_layout: vk::ImageLayout,
    pub array_layers: u32,
    pub mip_levels: u32,
    pub image_type: vk::ImageType,
    pub format: vk::Format,
    pub view_format: Option<vk::Format>,
    pub filter: vk::Filter
}

impl Default for ImageConfig {
    fn default() -> Self {
        ImageConfig {
            extent: vk::Extent3D { width: 0, height: 0, depth: 1 },
            image_usage_flags: vk::ImageUsageFlags::empty(),
            image_create_flags: vk::ImageCreateFlags::empty(),
            image_view_create_flags: vk::ImageViewCreateFlags::empty(),
            samples: vk::SampleCountFlags::TYPE_1,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            array_layers: 1,
            mip_levels: 1,
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            view_format: None,
            filter: vk::Filter::NEAREST
        }
    }
}

pub trait ImageRef {
    fn resolve_image<'a>() -> &'a Image;
}

pub trait ImageTrait: GpuResource {
    fn handle(&self) -> vk::Image;
    fn image_view(&self) -> vk::ImageView;
    fn sampler(&self) -> vk::Sampler;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn extent(&self) -> Extent2D {
        Extent2D { width: self.width(), height: self.height() }
    }
    fn binding(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo;
}

struct ImageInner {
    pub device_dep: Arc<DeviceInner>,
    pub allocator_dep: Option<Arc<Mutex<AllocatorInner>>>,
    pub(crate) image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
    pub allocation: Mutex<Option<Allocation>>,
    pub config: ImageConfig,
}

struct SwapchainImageInner {
    device_dep: Arc<DeviceInner>,
    image: vk::Image,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    extent: vk::Extent2D,
}

pub struct Image {
    inner: Arc<ImageInner>
}

pub struct SwapchainImage {
    inner: Arc<SwapchainImageInner>
}

impl Drop for SwapchainImageInner {
    fn drop(&mut self) {
        unsafe {
            self.device_dep.device.destroy_sampler(self.sampler, None);
            self.device_dep.device.destroy_image_view(self.image_view, None);
            trace!(target: LOG_TARGET, "Destroyed external image data: [{:?}]", self.image);
        }
    }
}

impl Drop for ImageInner {
    fn drop(&mut self) {
        unsafe {
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

impl GpuResource for Image {
    fn reference(&self) -> Arc<dyn Any> {
        self.inner.clone()
    }
}

impl GpuResource for SwapchainImage {
    fn reference(&self) -> Arc<dyn Any> {
        self.inner.clone()
    }
}

impl SwapchainImage {

    /**
     * Wrap an existing Vulkan image
     */
    pub fn from_raw(device: &Device, image: vk::Image, format: vk::Format, extent: Extent2D) -> SwapchainImage {
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
                .expect("Failed to create image view")
        };

        let sampler = unsafe {
            device.handle().create_sampler(&vk::SamplerCreateInfo::default(), None)
                .expect("Failed to create sampler")
        };

        SwapchainImage {
            inner: Arc::new(SwapchainImageInner {
                image,
                image_view,
                sampler,
                device_dep: device.inner.clone(),
                extent,
            })
        }
    }
}

impl Image {

    pub fn new(device: &Device, allocator: &mut Allocator, config: ImageConfig) -> Self {

        // Image
        let image_create_info = vk::ImageCreateInfo::default()
            .flags(config.image_create_flags)
            .extent(config.extent)
            .samples(config.samples)
            .usage(config.image_usage_flags)
            .sharing_mode(config.sharing_mode)
            .initial_layout(config.initial_layout)
            .array_layers(config.array_layers)
            .mip_levels(config.mip_levels)
            .image_type(config.image_type)
            .format(config.format);
        let image = unsafe {
            device.handle().create_image(&image_create_info, None)
                .expect("Failed to create image")
        };

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
            .flags(config.image_view_create_flags)
            .format(config.view_format.unwrap_or(config.format))
            .view_type(vk::ImageViewType::TYPE_2D)
            .image(image)
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
                .expect("Failed to create image view")
        };

        // Sampler
        let sampler_create_info = vk::SamplerCreateInfo::default()
            .mag_filter(config.filter)
            .min_filter(config.filter);
        let sampler = unsafe {
            device.handle().create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler")
        };

        trace!(target: LOG_TARGET, "Created image: [{:?}]", image);

        Self {
            inner: Arc::new(ImageInner {
                image,
                image_view,
                sampler,
                allocation: Mutex::new(Some(allocation)),
                device_dep: device.inner.clone(),
                allocator_dep: Some(allocator.inner.clone()),
                config
            })
        }
    }

    pub fn config(&self) -> ImageConfig {
        self.inner.config
    }
}

impl ImageTrait for Image {
    fn handle(&self) -> vk::Image {
        self.inner.image
    }

    fn image_view(&self) -> ImageView {
        self.inner.image_view
    }

    fn sampler(&self) -> Sampler {
        self.inner.sampler
    }

    fn width(&self) -> u32 {
        self.inner.config.extent.width
    }

    fn height(&self) -> u32 {
        self.inner.config.extent.height
    }

    fn binding(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .image_layout(layout)
            .image_view(self.inner.image_view)
            .sampler(self.inner.sampler)
    }
}

impl ImageTrait for SwapchainImage {
    fn handle(&self) -> vk::Image {
        self.inner.image
    }

    fn image_view(&self) -> ImageView {
        self.inner.image_view
    }

    fn sampler(&self) -> Sampler {
        self.inner.sampler
    }

    fn width(&self) -> u32 {
        self.inner.extent.width
    }

    fn height(&self) -> u32 {
        self.inner.extent.height
    }

    fn binding(&self, layout: ImageLayout) -> DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .image_layout(layout)
            .image_view(self.inner.image_view)
            .sampler(self.inner.sampler)
    }
}
