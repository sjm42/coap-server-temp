// url.rs

pub use coap_lite::ResponseType;

use std::fmt;
use std::fmt::{Debug, Display};
use std::{collections::HashMap, hash::Hash};

use crate::sensordata;

#[derive(Debug)]
pub struct MyResponse {
    code: ResponseType,
    data: String,
}

impl MyResponse {
    pub fn new<T: Into<String>>(code: ResponseType, data: T) -> MyResponse {
        MyResponse {
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

pub type UrlHandler = fn(&sensordata::MyData, Option<&str>) -> MyResponse;

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
            default: Self::resp_notfound,
        }
    }
    pub fn with_path<S>(mut self, urlpath: S, handler: UrlHandler) -> Self
    where
        S: Display + Into<String>,
    {
        self.add_path(urlpath, handler);
        self
    }
    pub fn clear(&mut self) -> &mut Self {
        self.map.clear();
        self.set_default(Self::resp_notfound)
    }
    pub fn set_default(&mut self, handler: UrlHandler) -> &mut Self {
        self.default = handler;
        self
    }
    pub fn add_path<S>(&mut self, urlpath: S, handler: UrlHandler) -> &mut Self
    where
        S: Display + Into<String>,
    {
        self.map.insert(urlpath.into(), handler);
        self
    }
    pub fn get_handler<S>(&self, urlpath: S) -> UrlHandler
    where
        S: Display + AsRef<str> + Hash + Eq,
    {
        match self.map.get(urlpath.as_ref()) {
            Some(handler) => *handler,
            None => self.default,
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    fn resp_notfound(_md: &sensordata::MyData, _payload: Option<&str>) -> MyResponse {
        MyResponse::new(ResponseType::NotFound, "NOT FOUND")
    }
}
// EOF
