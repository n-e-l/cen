use std::path::{PathBuf};
use std::time::Duration;
use log::error;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer};
use notify_debouncer_mini::DebouncedEventKind::Any;
use slotmap::{new_key_type, SlotMap};
use winit::event_loop::{EventLoopProxy};
use crate::app::app::UserEvent;
use crate::vulkan::{GraphicsPipelineConfig, ComputePipeline, Device, GraphicsPipeline, Pipeline, PipelineErr, ComputePipelineConfig};

new_key_type! { pub struct PipelineKey; }

pub enum PipelineHandle {
    Graphics(GraphicsPipelineConfig, GraphicsPipeline),
    Compute(ComputePipelineConfig, ComputePipeline),
}

pub trait IntoPipelineHandle {
    fn into_pipeline_handle(self, device: &Device) -> Result<PipelineHandle, PipelineErr>;
    fn shader_paths(&self) -> Vec<&PathBuf>;
}

impl IntoPipelineHandle for GraphicsPipelineConfig {
    fn into_pipeline_handle(self, device: &Device) -> Result<PipelineHandle, PipelineErr> {
        let pipeline = GraphicsPipeline::new(
            device,
            self.clone()
        )?;

        Ok(PipelineHandle::Graphics(self, pipeline))
    }

    fn shader_paths(&self) -> Vec<&PathBuf> {
        vec![&self.fragment_shader_source, &self.vertex_shader_source]
    }
}

impl IntoPipelineHandle for ComputePipelineConfig {
    fn into_pipeline_handle(self, device: &Device) -> Result<PipelineHandle, PipelineErr> {
        let pipeline = ComputePipeline::new(
            device,
            self.clone()
        )?;

        Ok(PipelineHandle::Compute(self, pipeline))
    }

    fn shader_paths(&self) -> Vec<&PathBuf> {
        vec![&self.shader_source]
    }
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
            ).expect("Failed to create file watcher");

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

    pub fn insert(&mut self, config: impl IntoPipelineHandle) -> Result<PipelineKey, PipelineErr> {

        // Watch for file changes
        config.shader_paths().iter().for_each(|path| {
            self.watcher.watcher().watch(path.as_path(), RecursiveMode::Recursive).unwrap_or_else(|_|{
                panic!("Failed to find path {:?}", path.as_path());
            });
        });

        Ok(self.pipelines.insert(config.into_pipeline_handle(&self.device)?))
    }

    pub fn get(&self, key: PipelineKey) -> Option<&dyn Pipeline> {
        self.pipelines.get(key)
            .map(|handle| {
                match handle {
                    PipelineHandle::Graphics(_, pipeline) => {
                        pipeline as &dyn Pipeline
                    }
                    PipelineHandle::Compute(_, pipeline) => {
                        pipeline as &dyn Pipeline
                    }
                }
            })
    }

    pub fn write(&mut self, key: PipelineKey, config: impl IntoPipelineHandle) -> Result<PipelineKey, PipelineErr> {
        *self.pipelines.get_mut(key).expect("Key not found") = config.into_pipeline_handle(&self.device)?;
        Ok(key)
    }

    pub fn reload(&mut self, path: &PathBuf) -> Result<(), PipelineErr> {
        // Look through all shaders with the given path and recreate them
        for (_, handle) in self.pipelines.iter_mut() {
            match handle {
                PipelineHandle::Graphics(config, pipeline) => {
                    if path.ends_with(&config.vertex_shader_source) || path.ends_with(&config.fragment_shader_source) {
                        *pipeline = GraphicsPipeline::new(
                            &self.device,
                            config.clone()
                        )?;
                    }
                }
                PipelineHandle::Compute(config, pipeline) => {
                    if path.ends_with(&config.shader_source) {
                        *pipeline = ComputePipeline::new(
                            &self.device,
                            config.clone()
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

}