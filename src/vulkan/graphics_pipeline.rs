use std::any::Any;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::Arc;
use ash::vk;
use ash::vk::{PushConstantRange, SampleCountFlags};
use log::trace;
use crate::vulkan::{DescriptorSetLayout, Device, GpuHandle, Pipeline, LOG_TARGET};
use crate::vulkan::device::DeviceInner;
use crate::vulkan::memory::GpuResource;
use crate::vulkan::pipeline::{create_shader_module, load_shader_code, PipelineErr};

#[derive(Clone)]
pub struct GraphicsPipelineConfig {
    pub color_formats: Vec<vk::Format>,
    pub depth_format: Option<vk::Format>,
    pub sample_count: SampleCountFlags,
    pub vertex_shader_source: PathBuf,
    pub fragment_shader_source: PathBuf,
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
    pub push_constant_ranges: Vec<PushConstantRange>,
    pub macros: HashMap<String, String>
}

pub struct GraphicsPipelineInner {
    pub pipeline_layout: vk::PipelineLayout,
    pub graphics_pipeline: vk::Pipeline,
    pub device_dep: Arc<DeviceInner>,
}

impl Drop for GraphicsPipelineInner {
    fn drop(&mut self) {
        unsafe {
            let graphics_pipeline_addr = format!("{:?}", self.graphics_pipeline);
            self.device_dep.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device_dep.device.destroy_pipeline_layout(self.pipeline_layout, None);
            trace!(target: LOG_TARGET, "Destroyed graphics pipeline: [{}]", graphics_pipeline_addr);
        }
    }
}

impl GpuHandle for GraphicsPipelineInner {}

#[derive(Clone)]
pub struct GraphicsPipeline {
    inner: Arc<GraphicsPipelineInner>
}

impl Pipeline for GraphicsPipeline {
    fn handle(&self) -> vk::Pipeline {
        self.inner.graphics_pipeline
    }

    fn bind_point(&self) -> vk::PipelineBindPoint {
        vk::PipelineBindPoint::GRAPHICS
    }

    fn layout(&self) -> vk::PipelineLayout {
        self.inner.pipeline_layout
    }

    fn resource(&self) -> &dyn GpuResource {
        self
    }
}

impl GpuResource for GraphicsPipeline {
    fn reference(&self) -> Arc<dyn Any> {
        self.inner.clone()
    }
}

impl GraphicsPipeline {

    pub fn new(
        device: &Device,
        config: GraphicsPipelineConfig
    ) -> Result<Self, PipelineErr> {

        // Dynamic rendering
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&config.color_formats)
            .depth_attachment_format(config.depth_format.unwrap_or(vk::Format::UNDEFINED));

        // Shaders
        let vertex_shader_code = load_shader_code(config.vertex_shader_source, &config.macros)?;
        let fragment_shader_code = load_shader_code(config.fragment_shader_source, &config.macros)?;

        let vertex_shader_module = create_shader_module(device.handle(), vertex_shader_code.to_vec());
        let fragment_shader_module = create_shader_module(device.handle(), fragment_shader_code.to_vec());

        let binding = CString::new("main").unwrap();
        let shader_stages = [
            // Vertex shader
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader_module)
                .name(binding.as_c_str()),
            // Fragment shader
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_module)
                .name(binding.as_c_str())
        ];

        // Multisample
        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(config.sample_count);

        // Viewport
        let viewports = [vk::Viewport::default()
            .width(512f32)
            .height(512f32)
            .x(0f32)
            .y(0f32)
        ];

        let scissors = [vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(vk::Extent2D::default().width(512).height(512))
        ];

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);

        // Vertex input
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default();

        // Input assembly
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // Rasterization
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .line_width(1.0);

        // Color blending
        let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);
        let color_blend_attachment_states = [color_blend_attachment_state];

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .attachments(&color_blend_attachment_states);

        // Depth stencil
        let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        // Layout
        let desc_layouts = config.descriptor_set_layouts
            .iter().map(|layout| layout.handle()).collect::<Vec<_>>();
        let create_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(&config.push_constant_ranges)
            .set_layouts(&desc_layouts);
        let pipeline_layout = unsafe {
            device.handle()
                .create_pipeline_layout(&create_info, None)
                .expect("Failed to create pipeline layout")
        };

        // pipeline
        let graphics_pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .push_next(&mut pipeline_rendering_create_info)
            .multisample_state(&multisample_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .color_blend_state(&color_blend_state)
            .rasterization_state(&rasterization_state)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .layout(pipeline_layout);

        let graphics_pipeline = unsafe {
            device.handle()
                .create_graphics_pipelines(vk::PipelineCache::null(), &[graphics_pipeline_create_info], None)
                .expect("Failed to create graphics pipeline")[0]
        };

        trace!(target: LOG_TARGET, "Created graphics pipeline: [{:?}]", graphics_pipeline);

        unsafe { device.handle().destroy_shader_module(fragment_shader_module, None); }
        unsafe { device.handle().destroy_shader_module(vertex_shader_module, None); }

        let pipeline_inner = GraphicsPipelineInner {
            pipeline_layout,
            graphics_pipeline,
            device_dep: device.inner.clone()
        };

        Ok(Self {
            inner: Arc::new(pipeline_inner)
        })
    }
}
