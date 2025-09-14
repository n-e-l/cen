use std::any::Any;
use std::sync::Arc;

pub trait GpuResource {
    fn reference(&self) -> Arc<dyn Any>;
}
pub trait GpuHandle {
}
