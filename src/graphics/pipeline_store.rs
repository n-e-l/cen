use std::collections::HashMap;
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ash::vk;
use log::error;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer};
use notify_debouncer_mini::DebouncedEventKind::Any;
use slotmap::{new_key_type, SlotMap};
use winit::event_loop::{EventLoopProxy};
use crate::app::app::UserEvent;
use crate::vulkan::{ComputePipeline, DescriptorSetLayout, Device, PipelineErr};

pub struct PipelineConfig {
    pub shader_path: PathBuf,
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
    pub push_constant_ranges: Vec<vk::PushConstantRange>,
    pub macros: HashMap<String, String>,
}

new_key_type! { pub struct PipelineKey; }

struct PipelineHandle {
    config: PipelineConfig,
    pipeline: ComputePipeline,
}

pub struct PipelineStore {
    device: Device,
    pipelines: SlotMap<PipelineKey, PipelineHandle>,
    watcher: Debouncer<RecommendedWatcher>,
}

impl PipelineStore {
    pub fn new(device: &Device, proxy: EventLoopProxy<UserEvent>) -> PipelineStore {

        // Register file watching for the shaders
        let watcher = notify_debouncer_mini::new_debouncer(
                Duration::from_millis(250),
                Self::watch_callback(proxy)
            ).expect("Failed to create file watcher")
        ;

        PipelineStore {
            watcher,
            device: device.clone(),
            pipelines: SlotMap::with_key(),
        }
    }

    fn watch_callback(event_loop_proxy: EventLoopProxy<UserEvent>) -> impl FnMut(DebounceEventResult) {
        move |event| match event {
            Ok(events) => {
                if let Some(e) = events
                    .iter().find(|e| e.kind == Any)
                {
                    event_loop_proxy.send_event(
                        UserEvent::GlslUpdate(e.path.clone())
                    ).expect("Failed to send event")
                }
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }

    pub fn update(&mut self, key: PipelineKey, config: PipelineConfig) -> Result<PipelineKey, PipelineErr> {
        // Watch for file changes
        self.watcher.watcher().watch(config.shader_path.as_path(), RecursiveMode::Recursive).unwrap();

        let pipeline = ComputePipeline::new(
            &self.device,
            config.shader_path.clone(),
            config.descriptor_set_layouts.as_slice(),
            config.push_constant_ranges.as_slice(),
            &config.macros
        )?;

        let handle = self.pipelines.get_mut(key).expect("Key not found");
        handle.config = config;
        handle.pipeline = pipeline;

        Ok(key)
    }

    pub fn insert_safe(&mut self, config: PipelineConfig) -> PipelineKey {
        // Watch for file changes
        self.watcher.watcher().watch(config.shader_path.as_path(), RecursiveMode::Recursive).unwrap();

        let pipeline = match ComputePipeline::new(
            &self.device,
            config.shader_path.clone(),
            config.descriptor_set_layouts.as_slice(),
            config.push_constant_ranges.as_slice(),
            &config.macros
        ) {
            Ok(p) => { p },
            Err(e) => {
                error!("{}", e);
                panic!()
            }
        };

        self.pipelines.insert(PipelineHandle {
            config,
            pipeline
        })
    }

    pub fn insert(&mut self, config: PipelineConfig) -> Result<PipelineKey, PipelineErr> {
        // Watch for file changes
        self.watcher.watcher().watch(config.shader_path.as_path(), RecursiveMode::Recursive).unwrap();

        let pipeline = ComputePipeline::new(
            &self.device,
            config.shader_path.clone(),
            config.descriptor_set_layouts.as_slice(),
            config.push_constant_ranges.as_slice(),
            &config.macros
        )?;

        Ok(self.pipelines.insert(PipelineHandle {
            config,
            pipeline
        }))
    }

    pub fn get(&self, key: PipelineKey) -> Option<ComputePipeline> {
        self.pipelines.get(key).map(|p| p.pipeline.clone())
    }

    pub fn reload(&mut self, path: &PathBuf) -> Result<(), PipelineErr> {
        // Look through all shaders with the given path and recreate them
        for handle in self.pipelines.iter_mut() {
            let config = &handle.1.config;
            if path.ends_with(&config.shader_path) {
                let pipeline = ComputePipeline::new(
                    &self.device,
                    config.shader_path.clone(),
                    config.descriptor_set_layouts.as_slice(),
                    config.push_constant_ranges.as_slice(),
                    &config.macros
                )?;
                handle.1.pipeline = pipeline;
            }
        }

        Ok(())
    }

}