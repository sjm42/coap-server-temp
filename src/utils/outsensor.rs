// utils/outsensor.rs

use log::*;
use std::lazy::*;
use std::sync::*;

const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";
static OUTSENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));

pub fn init() {
    info!("outsensor::init()");
    set(DEFAULT_OUTSENSOR);
}

pub fn get() -> String {
    let s = OUTSENSOR.lock().unwrap();
    // trace!("outsensor::get() --> {:?}", s);
    s.to_string()
}

pub fn set(data: &str) {
    trace!("outsensor::set({})", data);
    let mut s = OUTSENSOR.lock().unwrap();
    *s = data.to_string();
}
// EOF
