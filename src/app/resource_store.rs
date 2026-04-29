use std::sync::{Arc, RwLock};
use bitflags::bitflags;
use crate::app::TextureKey;
use crate::graphics::image_store::ImageKey;

bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct ImageFlags: u32 {
        const MATCH_SWAPCHAIN_EXTENT = 1 << 0;
    }
}

pub(crate) struct ImageData {
    pub(crate) image_key: ImageKey,
    pub(crate) texture_key: Option<TextureKey>
}

#[derive(Clone)]
pub struct ImageResource(pub(crate) Arc<RwLock<ImageData>>);

impl ImageResource {
    pub(crate) fn image_key(&self) -> ImageKey {
        self.0.read().unwrap().image_key.clone()
    }
    pub(crate) fn texture_key(&self) -> Option<TextureKey> {
        self.0.read().unwrap().texture_key.clone()
    }

    pub(crate) fn set_texture_key(&mut self, key: TextureKey) {
        self.0.write().unwrap().texture_key = Some(key);
    }
}

/// User-facing resources
pub struct ResourceStore {
    images: Vec<(ImageResource, ImageFlags)>
}

impl ResourceStore {
    pub fn insert(&mut self, image_key: ImageKey, flags: ImageFlags) -> ImageResource {
        let resource = ImageResource {
            0: Arc::new(RwLock::new(
                ImageData {
                    image_key,
                    texture_key: None
                }
            ))
        };

        self.images.push((resource.clone(), flags));

        resource
    }
}

impl ResourceStore {
    pub fn new() -> Self {
        Self {
            images: Vec::new()
        }
    }
}

impl ResourceStore {

    pub(crate) fn entries(&mut self) -> &Vec<(ImageResource, ImageFlags)> {
        &self.images
    }

    pub(crate) fn entries_mut(&mut self) -> &mut Vec<(ImageResource, ImageFlags)> {
        &mut self.images
    }

    pub fn cleanup(&mut self) {
        self.images.retain(|(resource, _flags)| {
            Arc::strong_count(&resource.0) > 1
        });
    }

    // pub fn recreate(&mut self, device: &Device, allocator: &mut Allocator, key: ImageKey, mut fun: impl FnMut(ImageConfig) -> ImageConfig) -> Result<(), &'static str> {
    //     if let Some(im) = self.images.get_mut(key) {
    //         let mut config = im.image.config();
    //         config = fun(config);
    //         *im = StoredImage{
    //             image: Image::new(device, allocator, config) ,
    //             flags: im.flags
    //         };
    //         return Ok(())
    //     }
    //
    //     Err("Key not found")
    // }
}