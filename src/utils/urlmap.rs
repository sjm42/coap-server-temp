// utils/urlmap.rs

use log::*;
use std::collections::HashMap;
use std::lazy::*;
use std::sync::*;

pub type UrlHandler = fn(Option<&str>) -> (String, String);
struct UrlMap {
    map: HashMap<&'static str, UrlHandler>,
    default: UrlHandler,
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

static URLMAP: SyncLazy<Mutex<UrlMap>> = SyncLazy::new(|| Mutex::new(UrlMap::new()));

fn resp_notfound(payload: Option<&str>) -> (String, String) {
    trace!(
        "UrlHandler::resp_notfound: payload={}",
        payload.unwrap_or(&"<none>".to_string())
    );
    ("4.04".to_string(), "NOT FOUND".to_string())
}

pub fn init() {
    trace!("urlmap::init()");
    set_default(resp_notfound);
    {
        let mut m = URLMAP.lock().unwrap();
        m.map.clear();
    }
}

pub fn len() -> usize {
    let m = URLMAP.lock().unwrap();
    m.map.len()
}

pub fn set_default(handler: UrlHandler) {
    let mut m = URLMAP.lock().unwrap();
    m.default = handler;
}

pub fn add(urlpath: &'static str, handler: UrlHandler) {
    trace!("urlmap::add({})", urlpath);
    let mut m = URLMAP.lock().unwrap();
    m.map.insert(urlpath, handler);
}

pub fn get(urlpath: &str) -> UrlHandler {
    let m = URLMAP.lock().unwrap();
    m.get(urlpath)
}
// EOF
