use std::sync::{Arc, Weak, RwLock};
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
#[derive(Clone)]
pub struct WeakImageResource(pub(crate) Weak<RwLock<ImageData>>);

impl ImageResource {
    pub(crate) fn new(image_key: ImageKey) -> Self {
        ImageResource(Arc::new(RwLock::new(ImageData { image_key, texture_key: None })))
    }

    pub(crate) fn downgrade(&self) -> WeakImageResource {
        WeakImageResource(Arc::downgrade(&self.0))
    }
}

impl ImageResource {
    pub(crate) fn image_key(&self) -> ImageKey {
        self.0.read().unwrap().image_key.clone()
    }
    pub(crate) fn texture_key(&self) -> Option<TextureKey> {
        self.0.read().unwrap().texture_key.clone()
    }
    pub(crate) fn set_image_key(&self, key: ImageKey) {
        self.0.write().unwrap().image_key = key;
    }
    pub(crate) fn set_texture_key(&self, key: TextureKey) {
        self.0.write().unwrap().texture_key = Some(key);
    }
}

impl WeakImageResource {
    pub(crate) fn upgrade(&self) -> Option<ImageResource> {
        self.0.upgrade().map(ImageResource)
    }
}

