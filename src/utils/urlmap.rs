// utils/urlmap.rs

use coap_lite::ResponseType;
use log::*;
use std::collections::HashMap;

pub type UrlHandler = fn(Option<&str>) -> (ResponseType, String);
pub struct UrlMap {
    map: HashMap<String, UrlHandler>,
    default: UrlHandler,
}

impl UrlMap {
    pub fn new() -> UrlMap {
        UrlMap {
            map: HashMap::with_capacity(10),
            default: resp_notfound,
        }
    }
    pub fn clear(&mut self) {
        self.map.clear();
        self.set_default(resp_notfound);
    }
    pub fn set_default(&mut self, handler: UrlHandler) {
        self.default = handler;
    }
    pub fn add(&mut self, urlpath: &str, handler: UrlHandler) {
        self.map.insert(urlpath.to_string(), handler);
    }
    pub fn get(&self, urlpath: &str) -> UrlHandler {
        match self.map.get(urlpath) {
            None => self.default,
            Some(handler) => *handler,
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn resp_notfound(payload: Option<&str>) -> (ResponseType, String) {
    trace!(
        "UrlHandler::resp_notfound: payload={}",
        payload.unwrap_or(&"<none>".to_string())
    );
    (ResponseType::NotFound, "NOT FOUND".to_string())
}
// EOF
