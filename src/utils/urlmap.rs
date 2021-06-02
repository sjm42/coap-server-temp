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

fn resp_notfound(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("UrlHandler::resp_notfound: payload={}", payload.unwrap_or(&"<none>".to_string()));
    *code = "4.04".to_string();
    *resp = "NOT FOUND".to_string();
}

pub fn init() {
    info!("urlmap::init()");
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
    info!("urlmap::add({})", urlpath);
    let mut m = URLMAP.lock().unwrap();
    m.map.insert(urlpath, handler);
}

pub fn get(urlpath: &str) -> UrlHandler {
    let m = URLMAP.lock().unwrap();
    m.get(urlpath)
}
// EOF
