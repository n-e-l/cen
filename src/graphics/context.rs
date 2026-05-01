use ash::vk::Queue;
use crate::app::{ImageFlags, ImageResource, WeakImageResource};
use crate::graphics::image_store::ImageStore;
use crate::graphics::pipeline_store::{IntoPipelineHandle, PipelineKey, PipelineStore};
use crate::vulkan::{Allocator, CommandPool, Device, Image, ImageConfig, Pipeline, PipelineErr};

pub struct GraphicsContext {
    pub command_pool: CommandPool,
    pub queue: Queue,
    pub allocator: Allocator,
    pub device: Device,
}

pub struct ImageContext {
    pub image_store: ImageStore,
    pub images: Vec<(WeakImageResource, ImageFlags)>,
}

impl ImageContext {

    pub fn create_image(&mut self, gfx: &mut GraphicsContext, config: ImageConfig, flags: ImageFlags) -> ImageResource {
        let image_key = self.image_store.insert(Image::new(&gfx.device, &mut gfx.allocator, config));
        let resource = ImageResource::new(image_key);
        self.images.push((resource.downgrade(), flags));
        resource
    }

    pub fn get(&self, resource: &ImageResource) -> &Image {
        self.image_store.get(&resource.image_key())
    }

    pub(crate) fn cleanup(&mut self) {
        self.images.retain(|(resource, _)| resource.upgrade().is_some());
        self.image_store.cleanup();
    }
}

pub struct PipelineContext {
    pub pipeline_store: PipelineStore,
}

impl PipelineContext {
    pub fn get(&self, key: PipelineKey) -> Option<&dyn Pipeline> {
        self.pipeline_store.get(key)
    }

    pub fn create_pipeline(&mut self, handle: impl IntoPipelineHandle) -> Result<PipelineKey, PipelineErr> {
        self.pipeline_store.insert(handle)
    }
}

#[cfg(test)]
mod tests {
    use ash::Entry;
    use ash::vk;
    use gpu_allocator::vulkan::AllocatorCreateDesc;
    use super::*;
    use crate::vulkan::{CommandPool, Device, ImageTrait, Instance};

    // PipelineContext is not tested here: PipelineStore::new requires a winit
    // EventLoopProxy, which needs a display connection unavailable in CI.

    fn make_graphics_context() -> (Entry, Instance, vk::PhysicalDevice, GraphicsContext) {
        let entry = Entry::linked();
        let instance = Instance::new(&entry, None);
        let (physical_device, queue_family_index) = instance.create_physical_device_headless();
        let device = Device::new(&instance, physical_device, queue_family_index);
        let queue = device.get_queue(0);
        let command_pool = CommandPool::new(&device, queue_family_index);
        let allocator = Allocator::new(
            &device,
            &AllocatorCreateDesc {
                instance: instance.handle().clone(),
                device: device.handle().clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
                allocation_sizes: Default::default(),
            },
        );
        let gfx = GraphicsContext { device, allocator, queue, command_pool };
        (entry, instance, physical_device, gfx)
    }

    #[test]
    fn create_graphics_context() {
        let (_entry, _instance, _physical_device, _gfx) = make_graphics_context();
    }

    #[test]
    fn image_context_create_image() {
        let (_entry, _instance, _physical_device, mut gfx) = make_graphics_context();
        let mut image_ctx = ImageContext { image_store: ImageStore::new(), images: Vec::new() };

        let config = ImageConfig {
            extent: vk::Extent3D { width: 64, height: 64, depth: 1 },
            image_usage_flags: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            ..Default::default()
        };

        let resource = image_ctx.create_image(&mut gfx, config, ImageFlags::empty());
        let image = image_ctx.get(&resource);
        assert_eq!(image.width(), 64);
        assert_eq!(image.height(), 64);
    }

    #[test]
    fn image_context_cleanup_drops_unreferenced_images() {
        let (_entry, _instance, _physical_device, mut gfx) = make_graphics_context();
        let mut image_ctx = ImageContext { image_store: ImageStore::new(), images: Vec::new() };

        let config = ImageConfig {
            extent: vk::Extent3D { width: 64, height: 64, depth: 1 },
            image_usage_flags: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            ..Default::default()
        };

        let resource = image_ctx.create_image(&mut gfx, config, ImageFlags::empty());
        assert_eq!(image_ctx.images.len(), 1);

        drop(resource);
        image_ctx.cleanup();
        assert_eq!(image_ctx.images.len(), 0);
    }
}
