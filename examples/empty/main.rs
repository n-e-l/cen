use cen::app::app::{Cen, AppConfig};
use cen::app::component::ComponentRegistry;

fn main() {
    Cen::run(
        AppConfig::default(),
        Box::new(|_| {
            ComponentRegistry::new()
        })
    );
}
