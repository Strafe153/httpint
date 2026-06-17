use reqwest::{
    Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use std::{rc::Rc, str::FromStr, sync::Arc, thread};

use crate::{
    AppWindow, Header, Response,
    request_service::{RequestService, ResponseData},
};

pub struct WindowHandler<'a> {
    window: &'a AppWindow,
    headers_model: Rc<VecModel<Header>>,
    client: Arc<RequestService>,
}

impl<'a> WindowHandler<'a> {
    pub fn init(window: &'a AppWindow) -> Self {
        let default_headers = vec![Header {
            name: "Accept".into(),
            value: "*/*".into(),
        }];

        let headers_model = Rc::new(VecModel::from(default_headers));
        window.set_headers(headers_model.clone().into());

        WindowHandler {
            window,
            headers_model,
            client: Arc::new(RequestService::new()),
        }
    }

    pub fn register_callbacks(&self) {
        self.on_add();
        self.on_edit_name();
        self.on_edit_value();
        self.on_remove();
        self.on_send();
    }

    fn on_add(&self) {
        let model = self.headers_model.clone();

        self.window.on_add(move || {
            let header = Header {
                name: SharedString::from(""),
                value: SharedString::from(""),
            };

            model.push(header);
        });
    }

    fn on_edit_name(&self) {
        let model = self.headers_model.clone();

        self.window.on_edit_name(move |i, value| {
            let i = i as usize;

            if let Some(mut header) = model.row_data(i) {
                header.name = value;
                model.set_row_data(i, header);
            }
        });
    }

    fn on_edit_value(&self) {
        let model = self.headers_model.clone();

        self.window.on_edit_value(move |i, value| {
            let i = i as usize;

            if let Some(mut header) = model.row_data(i) {
                header.value = value;
                model.set_row_data(i, header);
            }
        });
    }

    fn on_remove(&self) {
        let model = self.headers_model.clone();

        self.window.on_remove(move |i: i32| {
            model.remove(i as usize);
        });
    }

    fn on_send(&self) {
        let window = self.window.as_weak();
        let model = self.headers_model.clone();
        let client = self.client.clone();

        self.window.on_send(move |url, method, body| {
            window.unwrap().set_is_loading(true);

            let window_weak = window.clone();
            let client = client.clone();
            let headers = to_headers_vector(model.clone());

            thread::spawn(
                move || match perform_request(&client, method, url, body, headers) {
                    Ok(response) => set_success(window_weak, response),
                    Err(error) => set_failed(window_weak, error),
                },
            );
        });
    }
}

fn to_headers_vector(model: Rc<VecModel<Header>>) -> Vec<[String; 2]> {
    model
        .iter()
        .map(|h| [h.name.to_string(), h.value.to_string()])
        .collect()
}

fn to_header_map(headers: Vec<[String; 2]>) -> Result<HeaderMap, String> {
    let mut header_map = HeaderMap::new();

    for [name, value] in headers.iter() {
        let name =
            HeaderName::from_str(name).map_err(|_| format!("Invalid header name: {}", name))?;

        let value = HeaderValue::from_str(value)
            .map_err(|_| format!("Invalid header value for {}", name))?;

        header_map
            .try_append(name, value)
            .map_err(|_| "Maximum header size exceeded".to_string())?;
    }

    Ok(header_map)
}

fn create_response(response: ResponseData) -> Response {
    let headers: Vec<ModelRc<StandardListViewItem>> = response
        .headers
        .into_iter()
        .map(|vec| ModelRc::new(VecModel::from(vec)))
        .collect();

    Response {
        status_code: response.status_code,
        size: response.size.to_string().into(),
        headers: ModelRc::new(VecModel::from(headers)),
        body: response.body.into(),
    }
}

fn perform_request(
    client: &RequestService,
    method: SharedString,
    url: SharedString,
    body: SharedString,
    headers: Vec<[String; 2]>,
) -> Result<ResponseData, String> {
    let method =
        Method::from_str(&method).map_err(|_| format!("Incorrect method type: {}", method))?;
    let headers = to_header_map(headers)?;

    client.execute(method, url.to_string(), body.to_string(), headers)
}

fn set_success(window: Weak<AppWindow>, response_data: ResponseData) {
    window
        .upgrade_in_event_loop(move |w| {
            let response = create_response(response_data);

            w.set_response(response);
            w.set_error("".into());
            w.set_is_loading(false);
        })
        .unwrap();
}

fn set_failed(window: Weak<AppWindow>, error: String) {
    let error = error.to_string();

    window
        .upgrade_in_event_loop(move |w| {
            w.set_error(error.into());
            w.set_is_loading(false);
        })
        .unwrap();
}
