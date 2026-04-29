use std::sync::{Arc, Weak};
use slotmap::{new_key_type, SlotMap};
use crate::vulkan::{Device, Image};

new_key_type! { pub struct ImageId; }

pub struct StoredImage {
    pub image: Image,
    // A reference counted imageId, keeps track of external use of the image
    handle: Weak<ImageId>
}

#[derive(Clone)]
#[derive(Eq, Hash, PartialEq)]
pub struct ImageKey(Arc<ImageId>);

/// Manages internal images
pub struct ImageStore {
    images: SlotMap<ImageId, StoredImage>
}

impl ImageStore {
    pub(crate) fn new() -> Self {
        Self {
            images: SlotMap::default()
        }
    }
}

impl ImageStore {
    pub fn insert(&mut self, image: Image) -> ImageKey {
        let id = self.images.insert(StoredImage {
            image,
            handle: Weak::new()
        });

        let key: ImageKey = ImageKey(Arc::new(id));

        // Update the handle to do reference counting
        self.images.get_mut(id).unwrap().handle = Arc::downgrade(&key.0);

        key
    }

    pub fn get(&self, key: &ImageKey) -> &Image {
        self.images.get(*key.0).map(|s| &s.image).unwrap()
    }

    pub fn get_handle(&self, key: &ImageKey) -> Option<&StoredImage> {
        self.images.get(*key.0).map(|s| s)
    }

    pub fn cleanup(&mut self) {
        self.images.retain(|_, stored| {
            stored.handle.strong_count() > 0
        });
    }
}
