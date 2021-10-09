// utils/url.rs

pub use coap_lite::ResponseType;

use log::*;
use std::fmt;
use std::fmt::{Debug, Display};
use std::{collections::HashMap, hash::Hash};

use crate::sensordata;

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

pub type UrlHandler = fn(&sensordata::MyData, Option<&str>) -> UrlResponse;

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
        trace!("UrlMap::new_cap({})", cap);
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
        trace!("UrlMap::clear()");
        self.map.clear();
        self.set_default(Self::resp_notfound)
    }
    pub fn set_default(&mut self, handler: UrlHandler) -> &mut Self {
        trace!("UrlMap::set_default()");
        self.default = handler;
        self
    }
    pub fn add_path<S>(&mut self, urlpath: S, handler: UrlHandler) -> &mut Self
    where
        S: Display + Into<String>,
    {
        trace!("UrlMap::add_path({})", urlpath);
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
    fn resp_notfound(_md: &sensordata::MyData, payload: Option<&str>) -> UrlResponse {
        trace!(
            "UrlHandler::resp_notfound: payload={}",
            payload.unwrap_or("<none>")
        );
        UrlResponse::new(ResponseType::NotFound, "NOT FOUND")
    }
}
// EOF
