use crate::digitalocean::error::Error;
use reqwest::Method;
use reqwest::blocking::{ClientBuilder, RequestBuilder};
use serde::Deserialize;
use serde::de::DeserializeOwned;
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

    pub fn get_all_objects<R: DeserializeOwned, T, TE, LE>(
        &self,
        url: String,
        value_extractor: TE,
        link_extractor: LE,
    ) -> Result<Vec<T>, Error>
    where
        TE: Fn(R) -> Vec<T>,
        LE: Fn(&R) -> Links,
    {
        let mut url = url;
        let mut exit = false;
        let mut objects: Vec<T> = Vec::new();

        while !exit {
            let resp = self
                .get_request_builder(Method::GET, url.clone())
                .send()?
                .json::<R>()?;

            let links = link_extractor(&resp);
            objects.extend(value_extractor(resp).into_iter());
            if links.pages.is_some() && links.pages.clone().unwrap().next.is_some() {
                url = links.pages.unwrap().next.unwrap();
            } else {
                exit = true;
            }
        }

        Ok(objects)
    }

    pub fn get_object_by_name<R: DeserializeOwned, T, TE, LE, NE>(
        &self,
        name: &str,
        url: String,
        value_extractor: TE,
        link_extractor: LE,
        name_checker: NE,
    ) -> Result<Option<T>, Error>
    where
        TE: Fn(R) -> Vec<T>,
        LE: Fn(&R) -> Links,
        NE: Fn(&T, &str) -> bool,
    {
        let mut url = url;
        let mut exit = false;
        let mut obj: Option<T> = None;

        while !exit {
            let resp = self
                .get_request_builder(Method::GET, url.clone())
                .send()?
                .json::<R>()?;

            let links = link_extractor(&resp);
            obj = value_extractor(resp)
                .into_iter()
                .find(|v| name_checker(v, name));
            if obj.is_some() {
                exit = true;
            } else if links.pages.is_some() && links.pages.clone().unwrap().next.is_some() {
                url = links.pages.unwrap().next.unwrap();
            } else {
                exit = true;
            }
        }

        Ok(obj)
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

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
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
