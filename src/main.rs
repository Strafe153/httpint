use slint::{PlatformError, include_modules};
use window_handler::WindowHandler;

mod window_handler;

include_modules!();

fn main() -> Result<(), PlatformError> {
    let window = AppWindow::new()?;

    let handler = WindowHandler::init(&window);
    handler.register_callbacks();

    window.run()
}
