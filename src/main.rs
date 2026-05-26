use slint::{Model, PlatformError, SharedString, VecModel, include_modules};
use std::rc::Rc;

include_modules!();

fn main() -> Result<(), PlatformError> {
    let window = AppWindow::new()?;

    let headers_model = Rc::new(VecModel::from(vec![Header {
        name: SharedString::from("Accept"),
        value: SharedString::from("*/*"),
    }]));

    window.set_headers(headers_model.clone().into());

    window.on_send(|url, t| {
        println!("{}: {}", t, url);
    });

    let model_clone = headers_model.clone();
    window.on_remove(move |index: i32| {
        model_clone.remove(index as usize);
    });

    let model_clone = headers_model.clone();
    window.on_edit_name(move |i, value| {
        let i = i as usize;

        if let Some(mut header) = model_clone.row_data(i) {
            header.name = value;
            model_clone.set_row_data(i, header);
        }
    });

    let model_clone = headers_model.clone();
    window.on_edit_value(move |i, value| {
        let i = i as usize;

        if let Some(mut header) = model_clone.row_data(i) {
            header.value = value;
            model_clone.set_row_data(i, header);
        }
    });

    let model_clone = headers_model.clone();
    window.on_add(move || {
        model_clone.push(Header {
            name: SharedString::from(""),
            value: SharedString::from(""),
        });
    });

    window.run()
}