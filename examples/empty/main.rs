use cen::app::app::{App, AppConfig};
use cen::app::component::ComponentRegistry;

fn main() {
    App::run(
        AppConfig::default(),
        ComponentRegistry::new(),
    );
}
