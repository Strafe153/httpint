use reqwest::{Method, blocking::Client, header::HeaderMap};
use serde_json::{from_str, to_string_pretty};
use slint::StandardListViewItem;

pub struct RequestService {
    client: Client,
}

impl RequestService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub fn execute(
        &self,
        method: Method,
        url: String,
        body: String,
        headers: HeaderMap,
    ) -> Result<ResponseData, String> {
        let response = self
            .build_request(method, url, body, headers)
            .send()
            .map_err(|_| "Could not connect to the server")?;

        ResponseData::from_response(response)
    }

    fn build_request(
        &self,
        method: Method,
        url: String,
        body: String,
        headers: HeaderMap,
    ) -> reqwest::blocking::RequestBuilder {
        let mut request = self.client.request(method.clone(), url).headers(headers);

        if method != Method::GET && method != Method::DELETE {
            request = request.body(body);
        }

        request
    }
}

pub struct ResponseData {
    pub status_code: i32,
    pub size: u64,
    pub headers: Vec<Vec<StandardListViewItem>>,
    pub body: String,
}

impl ResponseData {
    fn from_response(response: reqwest::blocking::Response) -> Result<Self, String> {
        let status_code = response.status().as_u16() as i32;
        let t = response.content_length();
        println!("{:#?}", t);
        let size = response.content_length().unwrap_or(0);

        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                let value = value.to_str().ok()?;

                if value.is_empty() {
                    return None;
                }

                Some(vec![
                    StandardListViewItem::from(name.as_str()),
                    StandardListViewItem::from(value),
                ])
            })
            .collect();

        let body = response
            .text()
            .map_err(|_| "Could not read response body".to_string())?;

        let body = match from_str::<serde_json::Value>(&body) {
            Ok(value) => to_string_pretty(&value).unwrap_or(body),
            Err(_) => body,
        };

        let response = Self {
            status_code,
            size,
            headers,
            body,
        };

        Ok(response)
    }
}
