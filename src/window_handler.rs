use reqwest::{
    Method,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue, MaxSizeReached},
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use std::{rc::Rc, str::FromStr, sync::Arc, thread};

use crate::{AppWindow, Header, Response};

pub struct WindowHandler<'a> {
    window: &'a AppWindow,
    headers_model: Rc<VecModel<Header>>,
    // TODO: try making a wrapper around the client with the method to perform a request
    client: Arc<Client>,
}

impl<'a> WindowHandler<'a> {
    pub fn init(window: &'a AppWindow) -> Self {
        let headers_model = Rc::new(VecModel::from(vec![Header {
            name: "Accept".into(),
            value: "*/*".into(),
        }]));

        window.set_headers(headers_model.clone().into());

        WindowHandler {
            window,
            headers_model,
            client: Arc::new(Client::new()),
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

            let client = client.clone();
            let window_weak = window.clone();
            let headers = to_headers_vector(model.clone());

            thread::spawn(move || {
                let method = Method::from_str(&method).unwrap_or(Method::GET);
                let header_map = to_header_map(headers);

                match header_map {
                    Ok(h) => {
                        let response = create_request(client, method, url, body, h).send();

                        match response {
                            Ok(r) => {
                                let status_code = r.status().as_u16() as i32;
                                let size = r.content_length().unwrap_or(0).to_string();
                                let headers = read_response_headers(&r);

                                match r.text() {
                                    Ok(b) => {
                                        set_success(window_weak, status_code, size, headers, b)
                                    }
                                    Err(_) => {
                                        set_failed(window_weak, "Could not read response body")
                                    }
                                }
                            }
                            Err(_) => set_failed(window_weak, "Could not connect to the server"),
                        }
                    }
                    Err(_) => set_failed(window_weak, "Max size reached for headers"),
                }
            });
        });
    }
}

fn to_headers_vector(model: Rc<VecModel<Header>>) -> Vec<[String; 2]> {
    model
        .iter()
        .map(|h| [h.name.to_string(), h.value.to_string()])
        .collect()
}

fn to_header_map(headers: Vec<[String; 2]>) -> Result<HeaderMap, MaxSizeReached> {
    let mut header_map = HeaderMap::new();

    for [name, value] in headers.iter() {
        header_map.try_append(
            HeaderName::from_str(name).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        )?;
    }

    Ok(header_map)
}

fn create_request(
    client: Arc<Client>,
    method: Method,
    url: SharedString,
    body: SharedString,
    header_map: HeaderMap,
) -> reqwest::blocking::RequestBuilder {
    let mut request = client
        .request(method.clone(), url.to_string())
        .headers(header_map);

    if method != Method::GET && method != Method::DELETE {
        request = request.body(body.to_string());
    }

    request
}

fn read_response_headers(r: &reqwest::blocking::Response) -> Vec<Vec<StandardListViewItem>> {
    r.headers()
        .iter()
        .filter_map(|(name, value)| {
            let value = value.to_str().ok()?;

            if value.is_empty() {
                return None;
            }

            Some(vec![
                StandardListViewItem::from(SharedString::from(name.as_str())),
                StandardListViewItem::from(SharedString::from(value)),
            ])
        })
        .collect()
}

fn to_headers_vector_model(
    headers: Vec<Vec<StandardListViewItem>>,
) -> Vec<ModelRc<StandardListViewItem>> {
    headers
        .into_iter()
        .map(|vec| ModelRc::new(VecModel::from(vec)))
        .collect()
}

fn create_response(
    status_code: i32,
    size: String,
    headers: Vec<Vec<StandardListViewItem>>,
    body: String,
) -> Response {
    let headers = ModelRc::new(VecModel::from(to_headers_vector_model(headers)));

    Response {
        status_code,
        size: size.into(),
        headers,
        body: body.into(),
    }
}

fn set_success(
    window: Weak<AppWindow>,
    status_code: i32,
    size: String,
    headers: Vec<Vec<StandardListViewItem>>,
    body: String,
) {
    window
        .upgrade_in_event_loop(move |w| {
            let response = create_response(status_code, size, headers, body);

            w.set_response(response);
            w.set_is_loading(false);
        })
        .unwrap();
}

fn set_failed(window: Weak<AppWindow>, error: &str) {
    let error = error.to_string();

    window
        .upgrade_in_event_loop(move |w| {
            w.set_error(error.into());
            w.set_is_loading(false);
        })
        .unwrap();
}
