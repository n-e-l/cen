use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::WriteDescriptorSet;
use crate::vulkan::{CommandPool, Device, Framebuffer, GpuHandle, Image, Pipeline, RenderPass};
use crate::vulkan::device::DeviceInner;

pub struct CommandBufferInner {
    device_dep: Arc<DeviceInner>,
    command_buffer: vk::CommandBuffer,
    resource_handles: Mutex<Vec<Arc<dyn GpuHandle>>>,
}

pub struct CommandBuffer {
    inner: Arc<CommandBufferInner>,
}

impl CommandBuffer {
    pub fn new(device: &Device, command_pool: &CommandPool) -> CommandBuffer {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool.handle())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            device.handle()
                .allocate_command_buffers(&command_buffer_allocate_info)
                .map(|command_buffers| command_buffers[0])
                .expect("Failed to allocate command buffers")
        };

        CommandBuffer {
            inner: Arc::new(CommandBufferInner {
                device_dep: device.inner.clone(),
                command_buffer,
                resource_handles: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn begin(&mut self) {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default();
        unsafe {
            self.inner.device_dep.device
                .begin_command_buffer(self.inner.command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin command buffer");
        }

        // Reset resource handles
        self.inner.resource_handles.lock().expect("Failed to lock mutex").clear();
    }

    pub fn end(&self) {
        unsafe {
            self.inner.device_dep.device
                .end_command_buffer(self.inner.command_buffer)
                .expect("Failed to end command buffer");
        }
    }

    pub fn begin_render_pass(&self, render_pass: &RenderPass, framebuffer: &Framebuffer) {
        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: framebuffer.get_extent(),
            })
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }])
            .render_pass(render_pass.handle())
            .framebuffer(framebuffer.handle());
        unsafe {
            self.inner.device_dep.device
                .cmd_begin_render_pass(self.inner.command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
        }
    }
    
    pub fn begin_rendering(&self, rendering_info: &vk::RenderingInfoKHR<'_>) {
        unsafe {
            self.inner.device_dep.dynamic_rendering_loader
                .cmd_begin_rendering(self.inner.command_buffer, rendering_info);
        }
    }
    
    pub fn end_rendering(&self) {
        unsafe {
            self.inner.device_dep.dynamic_rendering_loader
                .cmd_end_rendering(self.inner.command_buffer);
        }
    }

    pub fn bind_push_descriptor_images(&self, pipeline: &dyn Pipeline, images: &Vec<&Image>) {
        let bindings = images.iter().map(|image| {
            vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::GENERAL)
                .image_view(image.image_view)
                .sampler(image.sampler)
        }).collect::<Vec<vk::DescriptorImageInfo>>();

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        unsafe {
            self.inner.device_dep.device_push_descriptor.cmd_push_descriptor_set(
                self.inner.command_buffer,
                pipeline.bind_point(),
                pipeline.layout(),
                0,
                &[write_descriptor_set]
            );
        }
    }

    pub fn bind_push_descriptor_image(&self, pipeline: &dyn Pipeline, image: &Image) {

        // TODO: Set bindings dynamically
        let bindings = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(image.image_view)
            .sampler(image.sampler)];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        unsafe {
            self.inner.device_dep.device_push_descriptor.cmd_push_descriptor_set(
                self.inner.command_buffer,
                pipeline.bind_point(),
                pipeline.layout(),
                0,
                &[write_descriptor_set]
            );
        }
    }

    pub fn bind_push_descriptor(&self, pipeline: &dyn Pipeline, set: u32, write_descriptor_sets: &[WriteDescriptorSet]) {
        unsafe {
            self.inner.device_dep.device_push_descriptor.cmd_push_descriptor_set(
                self.inner.command_buffer,
                pipeline.bind_point(),
                pipeline.layout(),
                set,
                write_descriptor_sets
            );
        }
    }

    pub fn end_render_pass(&self) {
        unsafe {
            self.inner.device_dep.device
                .cmd_end_render_pass(self.inner.command_buffer);
        }
    }

    pub fn push_constants(&self, pipeline: &dyn Pipeline, stage_flags: vk::ShaderStageFlags, offset: u32, data: &[u8]) {
        unsafe {
            self.inner.device_dep.device
                .cmd_push_constants(self.inner.command_buffer, pipeline.layout(), stage_flags, offset, data);
        }
    }

    pub fn set_viewport(&self, viewport: vk::Viewport) {
        unsafe {
            self.inner.device_dep.device
                .cmd_set_viewport(self.inner.command_buffer, 0, &[viewport]);
        }
    }

    pub fn set_scissor(&self, scissor: vk::Rect2D) {
        unsafe {
            self.inner.device_dep.device
                .cmd_set_scissor(self.inner.command_buffer, 0, &[scissor]);
        }
    }

    pub fn clear_color_image(&self, image: &Image) {
        unsafe {
            let mut clear_color_value = vk::ClearColorValue::default();
            clear_color_value.float32 = [ 0f32, 0f32, 0f32, 0f32];
            let sub_resource_ranges = [ vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .base_mip_level(0)
                .layer_count(1)
                .level_count(1) ];
            self.inner.device_dep.device
                .cmd_clear_color_image(
                    self.inner.command_buffer,
                    image.image,
                    vk::ImageLayout::GENERAL,
                    &clear_color_value,
                    &sub_resource_ranges
                )
        }
    }

    pub fn bind_pipeline(&mut self, pipeline: &dyn Pipeline) {
        unsafe {
            self.inner.device_dep.device
                .cmd_bind_pipeline(self.inner.command_buffer, pipeline.bind_point(), pipeline.handle());
        }
        self.inner.resource_handles.lock().expect("Failed to lock mutex").push(pipeline.reference())
    }

    pub fn dispatch(&self, x: u32, y: u32, z: u32) {
        unsafe {
            self.inner.device_dep.device
                .cmd_dispatch(self.inner.command_buffer, x, y, z);
        }
    }

    pub fn image_barrier(
        &self,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        dependency_flags: vk::DependencyFlags,
        image: &Image
    ) {
        unsafe {
            self.inner.device_dep.device
                .cmd_pipeline_barrier(
                    self.inner.command_buffer,
                    src_stage_mask,
                    dst_stage_mask,
                    dependency_flags,
                    &[],
                    &[],
                    &[vk::ImageMemoryBarrier::default()
                        .subresource_range(vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .base_mip_level(0)
                            .layer_count(1)
                            .level_count(1))
                        .old_layout(vk::ImageLayout::GENERAL)
                        .new_layout(vk::ImageLayout::GENERAL)
                        .src_access_mask(src_access_mask)
                        .dst_access_mask(dst_access_mask)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(*image.handle())
                    ]
                );
        }
    }

    pub fn bind_descriptor_sets(&self, pipeline: &dyn Pipeline, descriptor_sets: &[vk::DescriptorSet]) {
        unsafe {
            self.inner.device_dep.device
                .cmd_bind_descriptor_sets(
                    self.inner.command_buffer,
                    pipeline.bind_point(),
                    pipeline.layout(),
                    0,
                    descriptor_sets,
                    &[]
                );
        }
    }

    pub fn handle(&self) -> vk::CommandBuffer {
        self.inner.command_buffer
    }

    pub fn clone(&self) -> CommandBuffer {
        CommandBuffer {
            inner: self.inner.clone(),
        }
    }
}