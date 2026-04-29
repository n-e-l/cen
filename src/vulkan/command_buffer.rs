use std::any::Any;
use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::{BufferImageCopy, DeviceSize, FenceCreateFlags, ImageAspectFlags, ImageCopy, ImageLayout, ImageMemoryBarrier, WriteDescriptorSet};
use crate::vulkan::{Buffer, CommandPool, Device, Framebuffer, ImageTrait, Pipeline, RenderPass};
use crate::vulkan::device::DeviceInner;
use crate::vulkan::memory::GpuResource;

fn layout_stage_access(layout: vk::ImageLayout) -> (vk::PipelineStageFlags, vk::AccessFlags) {
    use vk::{PipelineStageFlags as S, AccessFlags as A};
    match layout {
        vk::ImageLayout::UNDEFINED | vk::ImageLayout::PREINITIALIZED =>
            (S::TOP_OF_PIPE,    A::empty()),
        vk::ImageLayout::GENERAL =>
            (S::ALL_COMMANDS,   A::MEMORY_READ | A::MEMORY_WRITE),
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL =>
            (S::TRANSFER,       A::TRANSFER_READ),
        vk::ImageLayout::TRANSFER_DST_OPTIMAL =>
            (S::TRANSFER,       A::TRANSFER_WRITE),
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL =>
            (S::ALL_COMMANDS,   A::SHADER_READ),
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL =>
            (S::COLOR_ATTACHMENT_OUTPUT, A::COLOR_ATTACHMENT_READ | A::COLOR_ATTACHMENT_WRITE),
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL =>
            (S::EARLY_FRAGMENT_TESTS | S::LATE_FRAGMENT_TESTS,
             A::DEPTH_STENCIL_ATTACHMENT_READ | A::DEPTH_STENCIL_ATTACHMENT_WRITE),
        vk::ImageLayout::PRESENT_SRC_KHR =>
            (S::BOTTOM_OF_PIPE, A::empty()),
        _ =>
            (S::ALL_COMMANDS,   A::MEMORY_READ | A::MEMORY_WRITE),
    }
}

pub struct CommandBufferInner {
    device_dep: Arc<DeviceInner>,
    command_buffer: vk::CommandBuffer,
    in_flight_fence: vk::Fence,
    resource_handles: Mutex<Vec<Arc<dyn Any>>>,
}

pub struct CommandBuffer {
    inner: Arc<CommandBufferInner>,
}

impl Drop for CommandBufferInner {
    fn drop(&mut self) {
        unsafe {
            self.device_dep.device.destroy_fence(self.in_flight_fence, None);
        }
    }
}

impl CommandBuffer {
    pub fn new(device: &Device, command_pool: &CommandPool, signaled: bool) -> CommandBuffer {
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

        let fence = unsafe {
            let fence_create_info = if signaled {
                vk::FenceCreateInfo::default()
                    .flags(FenceCreateFlags::SIGNALED)
            } else {
                vk::FenceCreateInfo::default()
            };

            device.handle().create_fence(&fence_create_info, None)
                .expect("Failed to create fence")
        };

        CommandBuffer {
            inner: Arc::new(CommandBufferInner {
                device_dep: device.inner.clone(),
                command_buffer,
                in_flight_fence: fence,
                resource_handles: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn track(&mut self, resource: &dyn GpuResource ) {
        let mut lock = self.inner.resource_handles.lock().expect("Failed to lock mutex");
        lock.push(resource.reference());
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

    pub fn begin_render_pass(&mut self, render_pass: &RenderPass, framebuffer: &Framebuffer) {
        self.track(render_pass);
        
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

    pub fn image_barriers<'a>(
        &mut self,
        images: &[&impl ImageTrait],
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_flags: vk::AccessFlags,
        dst_access_flags: vk::AccessFlags,
    )
    {
        images.iter().for_each(|image| self.track(*image));

        let image_memory_barriers = images.iter().map(|i| {

            vk::ImageMemoryBarrier::default()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_access_mask(src_access_flags)
                .dst_access_mask(dst_access_flags)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(i.handle())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
        }).collect::<Vec<ImageMemoryBarrier>>();
        unsafe {
            self.inner.device_dep.device.cmd_pipeline_barrier(
                self.inner.command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_memory_barriers
            )
        }
    }

    pub fn transition(&mut self, image: &impl ImageTrait, old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) {
        let (src_stage, src_access) = layout_stage_access(old_layout);
        let (dst_stage, dst_access) = layout_stage_access(new_layout);
        self.image_barrier(image, old_layout, new_layout, src_stage, dst_stage, src_access, dst_access);
    }

    pub fn image_barrier<'a>(
        &mut self,
        image: &impl ImageTrait,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_flags: vk::AccessFlags,
        dst_access_flags: vk::AccessFlags,
    )
    {
        self.track(image);

        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_access_mask(src_access_flags)
            .dst_access_mask(dst_access_flags)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image.handle())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        unsafe {
            self.inner.device_dep.device.cmd_pipeline_barrier(
                self.inner.command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier]
            )
        }
    }

    pub fn push_descriptor_set(&mut self, pipeline: &dyn Pipeline, set: u32, write_descriptor_sets: &[WriteDescriptorSet]) {
        self.track(pipeline.resource());

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

    pub fn bind_push_descriptor_images(&mut self, pipeline: &dyn Pipeline, images: &[&dyn ImageTrait]) {
        self.track(pipeline.resource());
        images.iter().for_each(|image| self.track(*image));

        let bindings = images.iter().map(|image| {
            vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::GENERAL)
                .image_view(image.image_view())
                .sampler(image.sampler())
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

    pub fn bind_push_descriptor_image(&mut self, pipeline: &dyn Pipeline, set: u32, image: &impl ImageTrait) {
        self.track(image);
        self.track(pipeline.resource());

        // TODO: Set bindings dynamically
        let bindings = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(image.image_view())
            .sampler(image.sampler())];

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
                set,
                &[write_descriptor_set]
            );
        }
    }

    pub fn bind_push_descriptor(&mut self, pipeline: &dyn Pipeline, set: u32, write_descriptor_sets: &[WriteDescriptorSet]) {
        self.track(pipeline.resource());

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

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.inner.device_dep.device
                .cmd_draw(self.inner.command_buffer, vertex_count, instance_count, first_vertex, first_instance);
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        _first_instance: u32,
    ) {
        unsafe {
            self.inner.device_dep.device
                .cmd_draw_indexed(self.inner.command_buffer, index_count, instance_count, first_index, vertex_offset, first_index);
        }
    }

    pub fn push_constants(&mut self, pipeline: &dyn Pipeline, stage_flags: vk::ShaderStageFlags, offset: u32, data: &[u8]) {
        self.track(pipeline.resource());

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

    pub fn clear_color_image_u32<'a>(&mut self, image: &impl ImageTrait, layout: ImageLayout, color: [u32; 4])
    {
        self.track(image);

        unsafe {
            let mut clear_color_value = vk::ClearColorValue::default();
            clear_color_value.uint32 = color;
            let sub_resource_ranges = [ vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .base_mip_level(0)
                .layer_count(1)
                .level_count(1) ];
            self.inner.device_dep.device
                .cmd_clear_color_image(
                    self.inner.command_buffer,
                    image.handle(),
                    layout,
                    &clear_color_value,
                    &sub_resource_ranges
                )
        }
    }

    pub fn clear_color_image<'a>(&mut self, image: &impl ImageTrait, layout: ImageLayout, color: [f32; 4])
    {
        self.track(image);

        unsafe {
            let mut clear_color_value = vk::ClearColorValue::default();
            clear_color_value.float32 = color;
            let sub_resource_ranges = [ vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .base_mip_level(0)
                .layer_count(1)
                .level_count(1) ];
            self.inner.device_dep.device
                .cmd_clear_color_image(
                    self.inner.command_buffer,
                    image.handle(),
                    layout,
                    &clear_color_value,
                    &sub_resource_ranges
                )
        }
    }

    pub fn blit_image<'a>(&mut self, src_image: &impl ImageTrait, src_layout: ImageLayout, dst_image: &impl ImageTrait, dst_layout: ImageLayout, regions: &[vk::ImageBlit], filter: vk::Filter)
    {
        self.track(src_image);
        self.track(dst_image);

        unsafe {
            self.inner.device_dep.device.cmd_blit_image(
                self.inner.command_buffer,
                src_image.handle(),
                src_layout,
                dst_image.handle(),
                dst_layout,
                regions,
                filter,
            );
        }
    }

    pub fn bind_pipeline(&mut self, pipeline: &dyn Pipeline) {
        self.track(pipeline.resource());

        unsafe {
            self.inner.device_dep.device
                .cmd_bind_pipeline(self.inner.command_buffer, pipeline.bind_point(), pipeline.handle());
        }
    }

    pub fn dispatch(&self, x: u32, y: u32, z: u32) {
        unsafe {
            self.inner.device_dep.device
                .cmd_dispatch(self.inner.command_buffer, x, y, z);
        }
    }
    
    pub fn fill_buffer(&mut self, buffer: &Buffer, offset: DeviceSize, size: DeviceSize, data: u32) {
        self.track(buffer);

        unsafe {
            self.inner.device_dep.device
                .cmd_fill_buffer(
                    self.inner.command_buffer,
                    *buffer.handle(),
                    offset,
                    size,
                    data
                );
        }
    }

    pub fn copy_buffer_to_image(&mut self, buffer: &Buffer, image: &impl ImageTrait, layout: ImageLayout, regions: &[BufferImageCopy])
    {
        self.track(buffer);
        self.track(image);

        unsafe {
            self.inner.device_dep.device
                .cmd_copy_buffer_to_image(
                    self.inner.command_buffer,
                    *buffer.handle(),
                    image.handle(),
                    layout,
                    regions
                );
        }
    }

    pub fn copy_image_to_buffer(&mut self, image: &impl ImageTrait, layout: ImageLayout, buffer: &Buffer, regions: &[BufferImageCopy]) {
        self.track(image);
        self.track(buffer);

        unsafe {
            self.inner.device_dep.device
                .cmd_copy_image_to_buffer(
                    self.inner.command_buffer,
                    image.handle(),
                    layout,
                    *buffer.handle(),
                    regions
                );
        }
    }
    
    pub fn copy_image(&mut self, from: &impl ImageTrait, from_layout: ImageLayout, to: &impl ImageTrait, to_layout: ImageLayout, regions: &[ImageCopy]) {
        self.track(from);
        self.track(to);

        unsafe {
            self.inner.device_dep.device
                .cmd_copy_image(
                    self.inner.command_buffer,
                    from.handle(),
                    from_layout,
                    to.handle(),
                    to_layout,
                    regions
                );
        }
    }
    
    pub fn buffer_barrier(
        &mut self,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        dependency_flags: vk::DependencyFlags,
        size: vk::DeviceSize,
        offset: vk::DeviceSize,
        buffer: &Buffer
    ) {
        self.track(buffer);

        unsafe {
            self.inner.device_dep.device
                .cmd_pipeline_barrier(
                    self.inner.command_buffer,
                    src_stage_mask,
                    dst_stage_mask,
                    dependency_flags,
                    &[],
                    &[vk::BufferMemoryBarrier::default()
                        .src_access_mask(src_access_mask)
                        .dst_access_mask(dst_access_mask)
                        .size(size)
                        .offset(offset)
                        .src_queue_family_index(0)
                        .dst_queue_family_index(0)
                        .buffer(*buffer.handle())
                    ],
                    &[]
                );
        }
    }

    pub fn bind_descriptor_sets(&mut self, pipeline: &dyn Pipeline, descriptor_sets: &[vk::DescriptorSet]) {
        self.track(pipeline.resource());

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

    pub fn fence(&self) -> vk::Fence {
        self.inner.in_flight_fence
    }

    pub fn clone(&self) -> CommandBuffer {
        CommandBuffer {
            inner: self.inner.clone(),
        }
    }
}