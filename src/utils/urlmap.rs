// utils/urlmap.rs

use std::collections::HashMap;
use log::*;
use std::lazy::*;
use std::sync::*;


pub type UrlHandler = fn(Option<&str>, &mut String, &mut String);
struct UrlMap {
    map: HashMap<&'static str, UrlHandler>,
    default: UrlHandler,
}


static URLMAP: SyncLazy<Mutex<UrlMap>> = SyncLazy::new(|| Mutex::new(UrlMap::new()));

fn resp_notfound(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("UrlHandler::resp_notfound: payload={}", payload.unwrap_or(&"<none>".to_string()));
    *code = "4.04".to_string();
    *resp = "NOT FOUND".to_string();
}

impl UrlMap {
    fn new() -> UrlMap {
        UrlMap {
            map: HashMap::new(),
            default: resp_notfound,
        }
    }
    fn get(&self, urlpath: &str) -> UrlHandler {
        match self.map.get(urlpath) {
            None => self.default,
            Some(handler) => *handler,
        }
    }
}

#[allow(dead_code)]
pub fn init() {
    let mut m = URLMAP.lock().unwrap();
    m.map.clear();
    m.default = resp_notfound;
}
#[allow(dead_code)]
pub fn len() -> usize {
    let m = URLMAP.lock().unwrap();
    m.map.len()
}
#[allow(dead_code)]
pub fn set_default(handler: UrlHandler) {
    let mut m = URLMAP.lock().unwrap();
    m.default = handler;
}
#[allow(dead_code)]
pub fn add(urlpath: &'static str, handler: UrlHandler) {
    let mut m = URLMAP.lock().unwrap();
    m.map.insert(urlpath, handler);
}
#[allow(dead_code)]
pub fn get(urlpath: &str) -> UrlHandler {
    let m = URLMAP.lock().unwrap();
    m.get(urlpath)
}

// EOF
