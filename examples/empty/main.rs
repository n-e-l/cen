use cen::app::app::{App, AppConfig};
use cen::app::component::ComponentRegistry;

fn main() {
    let registry = ComponentRegistry::new();

    App::run(
        AppConfig::default(),
        registry,
    );
}
