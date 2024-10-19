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

struct PipelineStoreInner {
    device: Device,
    pipelines: SlotMap<PipelineKey, PipelineHandle>,
    watcher: Debouncer<RecommendedWatcher>,
}

pub struct PipelineStore {
    inner: Arc<Mutex<PipelineStoreInner>>,
}

impl PipelineStore {
    pub fn new(device: &Device, proxy: EventLoopProxy<UserEvent>) -> PipelineStore {

        // Register file watching for the shaders
        let watcher = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(250),
            Self::watch_callback(proxy)
        ).expect("Failed to create file watcher");

        PipelineStore {
            inner: Arc::new(Mutex::new(PipelineStoreInner{
                watcher,
                device: device.clone(),
                pipelines: SlotMap::with_key(),
            }))
        }
    }

    fn watch_callback(event_loop_proxy: EventLoopProxy<UserEvent>) -> impl FnMut(DebounceEventResult) {
        move |event| match event {
            Ok(events) => {
                if let Some(e) = events
                    .iter()
                    .filter(|e| e.kind == Any)
                    .next()
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

    pub fn insert(&mut self, config: PipelineConfig) -> Result<PipelineKey, PipelineErr> {
        let mut inner = self.inner.lock().unwrap();

        // Watch for file changes
        inner.watcher.watcher().watch(config.shader_path.as_path(), RecursiveMode::Recursive).unwrap();

        let pipeline = ComputePipeline::new(
            &inner.device,
            config.shader_path.clone(),
            &config.descriptor_set_layouts.as_slice(),
            &config.push_constant_ranges.as_slice(),
            &config.macros
        )?;

        Ok(inner.pipelines.insert(PipelineHandle {
            config,
            pipeline
        }))
    }

    #[warn(dead_code)]
    pub fn get(&self, key: PipelineKey) -> Option<ComputePipeline> {
        self.inner.lock().unwrap().pipelines.get(key).map(|p| p.pipeline.clone())
    }

    pub fn reload(&mut self, path: &PathBuf) -> Result<(), PipelineErr> {
        let mut inner = self.inner.lock().unwrap();
        let device = inner.device.clone();

        // Look through all shaders with the given path and recreate them
        for handle in inner.pipelines.iter_mut() {
            let config = &handle.1.config;
            if path.ends_with(&config.shader_path) {
                let pipeline = ComputePipeline::new(
                    &device,
                    config.shader_path.clone(),
                    &config.descriptor_set_layouts.as_slice(),
                    &config.push_constant_ranges.as_slice(),
                    &config.macros
                )?;
                handle.1.pipeline = pipeline;
            }
        }

        Ok(())
    }

}