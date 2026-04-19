use slotmap::{new_key_type, SlotMap};
use crate::vulkan::{Allocator, Device, Image, ImageConfig};

new_key_type! { pub struct ImageKey; }

pub struct ImageStore {
    images: SlotMap<ImageKey, Image>
}

impl Default for ImageStore {
    fn default() -> Self {
        Self {
            images: SlotMap::default()
        }
    }
}

impl ImageStore {
    pub fn insert(&mut self, image: Image) -> ImageKey {
        self.images.insert(image)
    }

    pub fn get(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key)
    }

    pub fn recreate(&mut self, device: &Device, allocator: &mut Allocator, key: ImageKey, mut fun: impl FnMut(ImageConfig) -> ImageConfig) -> Result<(), &'static str> {
        if let Some(im) = self.images.get_mut(key) {
            let mut config = im.config();
            config = fun(config);
            *im = Image::new(device, allocator, config);
            return Ok(())
        }

        Err("Key not found")
    }
}