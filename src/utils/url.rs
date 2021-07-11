// utils/url.rs

pub use coap_lite::ResponseType;
use log::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct UrlResponse {
    code: ResponseType,
    data: String,
}

#[allow(dead_code)]
impl UrlResponse {
    pub fn new<T: Into<String>>(code: ResponseType, data: T) -> UrlResponse {
        UrlResponse {
            code,
            data: data.into(),
        }
    }
    pub fn code(&self) -> ResponseType {
        self.code
    }
    pub fn data(&self) -> &str {
        self.data.as_str()
    }
}

pub type UrlHandler = fn(Option<&str>) -> UrlResponse;

pub struct UrlMap {
    map: HashMap<String, UrlHandler>,
    default: UrlHandler,
}

impl fmt::Debug for UrlMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UrlMap")
            .field("default", &format!("{:p}", self.default as *const ()))
            .field(
                "map",
                &self
                    .map
                    .iter()
                    .map(|(k, v)| format!("{}->{:p}", k, *v as *const ()))
                    .collect::<Vec<String>>()
                    .join(", "),
            )
            .finish()
    }
}

#[allow(dead_code)]
impl UrlMap {
    pub fn new() -> UrlMap {
        UrlMap::new_cap(8)
    }
    pub fn new_cap(cap: usize) -> UrlMap {
        UrlMap {
            map: HashMap::with_capacity(cap),
            default: resp_notfound,
        }
    }
    pub fn with_map<T: Into<String>>(mut self, urlpath: T, handler: UrlHandler) -> Self {
        self.add_map(urlpath, handler);
        self
    }
    pub fn clear(&mut self) -> &mut Self {
        self.map.clear();
        self.set_default(resp_notfound)
    }
    pub fn set_default(&mut self, handler: UrlHandler) -> &mut Self {
        self.default = handler;
        self
    }
    pub fn add_map<T: Into<String>>(&mut self, urlpath: T, handler: UrlHandler) -> &mut Self {
        self.map.insert(urlpath.into(), handler);
        self
    }
    pub fn get(&self, urlpath: &str) -> UrlHandler {
        match self.map.get(urlpath) {
            Some(handler) => *handler,
            None => self.default,
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn resp_notfound(payload: Option<&str>) -> UrlResponse {
    trace!(
        "UrlHandler::resp_notfound: payload={}",
        payload.unwrap_or("<none>")
    );
    UrlResponse::new(ResponseType::NotFound, "NOT FOUND")
}
// EOF
