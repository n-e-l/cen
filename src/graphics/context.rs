use std::sync::Arc;
use ash::vk::Queue;
use crate::app::{ImageFlags, ImageResource};
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
    pub images: Vec<(ImageResource, ImageFlags)>,
}

impl ImageContext {

    pub(crate) fn create(&mut self, gfx: &mut GraphicsContext, config: ImageConfig, flags: ImageFlags) -> ImageResource {
        let image_key = self.image_store.insert(Image::new(&gfx.device, &mut gfx.allocator, config));
        let resource = ImageResource::new(image_key);
        self.images.push((resource.clone(), flags));
        resource
    }

    pub fn get(&self, resource: &ImageResource) -> &Image {
        self.image_store.get(&resource.image_key())
    }

    pub(crate) fn cleanup(&mut self) {
        self.images.retain(|(resource, _)| Arc::strong_count(&resource.0) > 1);
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
