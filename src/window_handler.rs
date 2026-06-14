use reqwest::{
    Method,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, StandardListViewItem, VecModel};
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

            let headers: Vec<_> = model
                .iter()
                .map(|h| [h.name.to_string(), h.value.to_string()])
                .collect();

            let client = client.clone();
            let window_weak_clone = window_weak.clone();

            thread::spawn(move || {
                let mut header_map = HeaderMap::new();

                for [name, value] in headers.iter() {
                    let result = header_map.try_append(
                        HeaderName::from_str(name).unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );

                    match result {
                        Ok(_) => break,
                        Err(_) => {}
                    }
                }

                if let Ok(method) = Method::from_str(&method)
                    && !url.is_empty()
                {
                    let mut request = client
                        .request(method.clone(), url.as_str())
                        .headers(header_map);

                    if method != Method::GET && method != Method::DELETE {
                        request = request.body(body.to_string());
                    }

                    let response_result = request.send();

                    match response_result {
                        Ok(r) => {
                            let status_code = r.status().as_u16() as i32;
                            let size = r.content_length().unwrap_or(0).to_string();

                            let headers: Vec<Vec<StandardListViewItem>> = r
                                .headers()
                                .iter()
                                .map(|(name, value)| {
                                    vec![
                                        StandardListViewItem::from(SharedString::from(
                                            name.as_str(),
                                        )),
                                        StandardListViewItem::from(SharedString::from(
                                            value.to_str().unwrap_or(""),
                                        )),
                                    ]
                                })
                                .collect();

                            if let Ok(text) = r.text() {
                                window_weak_clone
                                    .upgrade_in_event_loop(move |window| {
                                        let headers: Vec<ModelRc<StandardListViewItem>> = headers
                                            .into_iter()
                                            .map(|vec| ModelRc::new(VecModel::from(vec)))
                                            .collect();

                                        let response = Response {
                                            status_code,
                                            size: size.into(),
                                            headers: ModelRc::new(VecModel::from(headers)),
                                            body: text.into(),
                                        };

                                        window.set_response(response);
                                        window.set_is_loading(false);
                                    })
                                    .unwrap();
                            }
                        }
                        Err(_) => {
                            window_weak_clone
                                .upgrade_in_event_loop(move |window| window.set_is_loading(false))
                                .unwrap();
                        }
                    }
                }
            });
        });
    }
}
