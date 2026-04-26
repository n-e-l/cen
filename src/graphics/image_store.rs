use ash::vk::Extent2D;
use bitflags::bitflags;
use slotmap::{new_key_type, SlotMap};
use crate::vulkan::{Allocator, Device, Image, ImageConfig};

bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct ImageFlags: u32 {
        const MATCH_SWAPCHAIN_EXTENT = 1 << 0;
    }
}

struct StoredImage {
    image: Image,
    flags: ImageFlags
}

new_key_type! { pub struct ImageKey; }
pub struct ImageStore {
    images: SlotMap<ImageKey, StoredImage>
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
        self.images.insert(StoredImage {
            image,
            flags: ImageFlags::empty()
        })
    }

    pub fn insert_with_flags(&mut self, flags: ImageFlags, image: Image) -> ImageKey {
        self.images.insert(StoredImage {
            image,
            flags
        })
    }

    pub fn get(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key).map(|s| &s.image)
    }

    /**
     * Update all images with the set flags
     * @returns a list of updated image keys
     */
    pub(crate) fn on_swapchain_resize(&mut self, device: &Device, allocator: &mut Allocator, extent: Extent2D) -> Vec<ImageKey> {
        self.images.iter_mut().filter_map(|(key, si)| {
           if si.flags.contains(ImageFlags::MATCH_SWAPCHAIN_EXTENT) {
               let mut config = si.image.config();
               config.extent.width = extent.width;
               config.extent.height = extent.height;
               si.image = Image::new(device, allocator, config);
               return Some(key)
           }
           None
        }).collect::<Vec<_>>()
    }

    pub fn recreate(&mut self, device: &Device, allocator: &mut Allocator, key: ImageKey, mut fun: impl FnMut(ImageConfig) -> ImageConfig) -> Result<(), &'static str> {
        if let Some(im) = self.images.get_mut(key) {
            let mut config = im.image.config();
            config = fun(config);
            *im = StoredImage{
                image: Image::new(device, allocator, config) ,
                flags: im.flags
            };
            return Ok(())
        }

        Err("Key not found")
    }
}