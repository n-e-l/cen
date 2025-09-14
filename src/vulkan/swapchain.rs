use std::sync::Arc;
use ash::khr::swapchain;
use ash::vk;
use ash::vk::{CompositeAlphaFlagsKHR, ImageUsageFlags, PresentModeKHR, SharingMode, SurfaceFormatKHR, SwapchainKHR};
use log::{debug, info};
use crate::graphics::renderer::WindowState;
use crate::vulkan;
use crate::vulkan::{Device, Image, Instance, Surface, LOG_TARGET};
use crate::vulkan::device::DeviceInner;

/// Vulkan does not have a concept of a "default framebuffer". Instead, we need a framework that "owns" the images that will eventually be presented to the screen.
/// The general purpose of the swapchain is to synchronize the presentation of images with the refresh rate of the screen.
pub struct SwapchainInner {
    #[allow(dead_code)]
    device_dep: Arc<DeviceInner>,
    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,
    images: Vec<vulkan::Image>,
    extent: vk::Extent2D,
    format: SurfaceFormatKHR
}

impl Drop for SwapchainInner {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.swapchain, None)
        }
    }
}

pub struct Swapchain {
    pub inner: Arc<SwapchainInner>,
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        device: &Device,
        window: &WindowState,
        surface: &Surface,
        preferred_present_mode: PresentModeKHR,
        old_swapchain: Option<SwapchainKHR>
    ) -> Swapchain {
        let swapchain_loader = swapchain::Device::new(instance.handle(), device.handle());

        let available_formats = surface.get_formats(physical_device);
        let surface_format = available_formats.iter()
            .find(|f| {
                #[cfg(any(target_os = "linux", target_os = "windows"))]
                let preferred_format = &&vk::SurfaceFormatKHR {
                    format: vk::Format::R8G8B8A8_SRGB,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                };
                
                #[cfg(target_os = "macos")]
                let preferred_format = &&vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_SRGB,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                };
                
                f == preferred_format
            })
            .unwrap_or(available_formats.first().expect("No surface format found"));

        info!(target: LOG_TARGET, "Using swapchain surface format: {:?}", surface_format);

        let surface_capabilities = surface.get_surface_capabilities(physical_device);

        let mut desired_image_count = surface_capabilities.min_image_count + 0;
        // Max image count can be 0
        if surface_capabilities.max_image_count > 0 && desired_image_count > surface_capabilities.max_image_count {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let pre_transform = if surface_capabilities.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY) {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let present_modes = surface.get_present_modes(physical_device);
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == preferred_present_mode)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        
        debug!(target: LOG_TARGET, "Present mode: {:?}", present_mode);

        let extent = match surface_capabilities.current_extent.width {
            u32::MAX => window.extent2d,
            _ => surface_capabilities.current_extent
        };
        info!(target: LOG_TARGET, "Using swapchain extent: {:?}", extent);
        info!(target: LOG_TARGET, "Using scale factor: {:?}", window.scale_factor);
        info!(target: LOG_TARGET, "Using image count: {:?}", desired_image_count);

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::TRANSFER_DST)
            .image_extent(extent)
            .image_sharing_mode(SharingMode::EXCLUSIVE)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .pre_transform(pre_transform)
            .present_mode(present_mode)
            .min_image_count(desired_image_count)
            .surface(*surface.handle())
            .clipped(true)
            .image_array_layers(1);

        if let Some(old_swapchain) = old_swapchain {
            create_info = create_info.old_swapchain(old_swapchain);
        }

        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None).unwrap() };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() }.iter()
            .map(|&image| vulkan::Image::from_raw(device, image, surface_format.format, extent))
            .collect::<Vec<vulkan::Image>>();

        let swapchain_inner = SwapchainInner {
            device_dep: device.inner.clone(),
            swapchain_loader,
            swapchain,
            images,
            extent,
            format: *surface_format
        };

        Self {
            inner: Arc::new(swapchain_inner)
        }
    }

    pub fn get_images(&self) -> &Vec<Image> {
        &self.inner.images
    }

    pub fn get_image_views(&self) -> Vec<vk::ImageView> {
        self.inner.images.iter().map(|i| i.image_view()).collect()
    }

    pub fn get_image_count(&self) -> u32 {
        self.inner.images.len() as u32
    }

    pub fn get_extent(&self) -> vk::Extent2D {
        self.inner.extent
    }

    pub fn get_format(&self) -> SurfaceFormatKHR {
        self.inner.format
    }

    pub fn handle(&self) -> SwapchainKHR {
        self.inner.swapchain
    }

    /// Queue an image for presentation.
    ///
    /// - `semaphore` - A semapore to wait on before issuing the present info.
    /// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkQueuePresentKHR.html
    pub fn queue_present(&self, queue: vk::Queue, wait_semaphore: vk::Semaphore, image_index: u32) {
        let mut result = [vk::Result::SUCCESS];
        unsafe {
            let swapchains = [self.handle()];
            let indices = [image_index];
            let semaphores = [wait_semaphore];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&semaphores)
                .swapchains(&swapchains)
                .image_indices(&indices)
                .results(&mut result);
            self.inner.swapchain_loader.queue_present(queue, &present_info)
                .expect("Failed to present queue");
        }
    }

    /// Acquire the next image in the swapchain.
    /// * `semaphore` - A semaphore to signal when the image is available.
    ///
    /// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkAcquireNextImageKHR.html
    pub fn acquire_next_image(&self, semaphore: vk::Semaphore) -> u32 {
        unsafe {
            let (image_index, _) = self.inner.swapchain_loader
                .acquire_next_image(
                    self.handle(),
                    u64::MAX,
                    semaphore,
                    vk::Fence::null()
                )
                .expect("Failed to acquire next image");
            image_index
        }
    }
}
