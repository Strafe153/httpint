use slint::{Model, PlatformError, SharedString, VecModel, include_modules};
use std::rc::Rc;

include_modules!();

fn main() -> Result<(), PlatformError> {
    let window = AppWindow::new()?;

    set_initial_headers(&window);

    window.on_send(|url, t| {
        println!("{}: {}", t, url);
    });

    window.run()
}

fn set_initial_headers(window: &AppWindow) {
    let initial_headers = vec![Header {
        name: SharedString::from("Accept"),
        value: SharedString::from("*/*"),
    }];

    let mut headers: Vec<Header> = window.get_headers().iter().collect();
    headers.extend(initial_headers);

    let headers_model = Rc::new(VecModel::from(headers));
    window.set_headers(headers_model.into());
}
