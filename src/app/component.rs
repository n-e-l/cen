use std::sync::{Arc, Mutex};
use crate::app::gui::GuiComponent;
use crate::graphics::renderer::RenderComponent;

pub enum Component {
    Render(Arc<Mutex<dyn RenderComponent>>),
    Gui(Arc<Mutex<dyn GuiComponent>>)
}

pub struct ComponentRegistry {
    storage: Vec<Component>
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self { storage: Vec::new() }
    }

    pub fn register(mut self, component: Component) -> Self
    {
        self.storage.push(component);
        self
    }

    pub fn render_components(&self) -> Vec<Arc<Mutex<dyn RenderComponent>>> {
        self.storage.iter().filter_map(|c| match c {
            Component::Render(c) => Some(c.clone()),
            _ => None
        }).collect()
    }

    pub fn gui_components(&self) -> Vec<Arc<Mutex<dyn GuiComponent>>> {
        self.storage.iter().filter_map(|c| match c {
            Component::Gui(c) => Some(c.clone()),
            _ => None
        }).collect()
    }
}
