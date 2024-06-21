use reqwest::blocking::{ClientBuilder, RequestBuilder};
use reqwest::Method;
use serde::Deserialize;
use url::Url;

#[derive(Clone)]
pub struct DigitalOceanApiClient {
    base_url: Url,
    force_https: bool,
    token: String,
}

impl DigitalOceanApiClient {
    pub fn new(token: String) -> DigitalOceanApiClient {
        DigitalOceanApiClient {
            base_url: Url::parse("https://api.digitalocean.com").unwrap(),
            force_https: true,
            token,
        }
    }

    pub fn get_url(&self, endpoint: &str) -> String {
        self.base_url.join(endpoint).unwrap().to_string()
    }

    pub fn get_request_builder(&self, method: Method, url: String) -> RequestBuilder {
        let mut real_url = url;
        if self.force_https {
            real_url = real_url.replace("http://", "https://");
        }

        ClientBuilder::new()
            .build()
            .unwrap()
            .request(method, real_url)
            .header("Authorization", format!("Bearer {}", self.token))
    }

    #[cfg(test)]
    pub fn new_for_test(token: String, base_url: String) -> DigitalOceanApiClient {
        DigitalOceanApiClient {
            base_url: Url::parse(base_url.as_str()).unwrap(),
            force_https: false,
            token,
        }
    }
}

// common parts of responses for collections

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Meta {
    pub total: u32,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Links {
    pub pages: Option<Pages>,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct Pages {
    pub first: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
    pub last: Option<String>,
}

// common error message format

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ErrorResponse {
    pub id: String,
    pub message: String,
    pub request_id: Option<String>,
}
