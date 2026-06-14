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

    // TODO: refactor this by splitting the logic into separate methods
    // TODO: properly handle errors
    fn on_send(&self) {
        let window_weak = self.window.as_weak();
        let model = self.headers_model.clone();
        let client = self.client.clone();

        self.window.on_send(move |url, method, body| {
            window_weak.unwrap().set_is_loading(true);

            let client = client.clone();
            let window_weak_clone = window_weak.clone();
            let headers = to_headers_vector(model.clone());

            thread::spawn(move || {
                // rewrite with a match clause
                if let Ok(method) = Method::from_str(&method)
                    && !url.is_empty()
                {
                    let header_map_result = to_header_map(headers);

                    match header_map_result {
                        Ok(h) => {
                            let response_result = prepare_request(
                                client,
                                method,
                                url.as_str(),
                                body.to_string(),
                                h,
                            )
                            .send();

                            match response_result {
                                Ok(r) => {
                                    let status_code = r.status().as_u16() as i32;
                                    let size = r.content_length().unwrap_or(0).to_string();
                                    let headers = get_response_headers(&r);

                                    // rewrite with a match clause
                                    if let Ok(text) = r.text() {
                                        set_success(
                                            window_weak_clone,
                                            status_code,
                                            size,
                                            headers,
                                            text,
                                        );
                                    }
                                }
                                Err(_) => set_failed(window_weak_clone),
                            }
                        }
                        Err(_) => set_failed(window_weak_clone),
                    }
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

fn prepare_request(
    client: Arc<Client>,
    method: Method,
    url: &str,
    body: String,
    header_map: HeaderMap,
) -> reqwest::blocking::RequestBuilder {
    let mut request = client.request(method.clone(), url).headers(header_map);

    if method != Method::GET && method != Method::DELETE {
        request = request.body(body);
    }

    request
}

fn get_response_headers(r: &reqwest::blocking::Response) -> Vec<Vec<StandardListViewItem>> {
    r.headers()
        .iter()
        .map(|(name, value)| {
            vec![
                StandardListViewItem::from(SharedString::from(name.as_str())),
                StandardListViewItem::from(SharedString::from(value.to_str().unwrap_or(""))),
            ]
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
    text: String,
) -> Response {
    let headers = ModelRc::new(VecModel::from(to_headers_vector_model(headers)));

    Response {
        status_code,
        size: size.into(),
        headers,
        body: text.into(),
    }
}

fn set_success(
    window: Weak<AppWindow>,
    status_code: i32,
    size: String,
    headers: Vec<Vec<StandardListViewItem>>,
    text: String,
) {
    window
        .upgrade_in_event_loop(move |w| {
            let response = create_response(status_code, size, headers, text);

            w.set_response(response);
            w.set_has_response_error(false);
            w.set_is_loading(false);
        })
        .unwrap();
}

fn set_failed(window: Weak<AppWindow>) {
    // substitute has_response_error with error_message and here pass
    // the message about header map error
    window
        .upgrade_in_event_loop(move |w| {
            w.set_has_response_error(true);
            w.set_is_loading(false);
        })
        .unwrap();
}
