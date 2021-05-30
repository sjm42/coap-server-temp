// utils/outsensor.rs

use std::lazy::*;
use std::sync::*;
use log::*;

#[allow(dead_code)]
const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";
static OUTSENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));

#[allow(dead_code)]
pub fn init() {
    trace!("outsensor::init() called");
    set(DEFAULT_OUTSENSOR);
}
#[allow(dead_code)]
pub fn get() -> String {
    let s = OUTSENSOR.lock().unwrap();
    // trace!("outsensor::get() --> {:?}", s);
    s.to_string()
}
#[allow(dead_code)]
pub fn set(data: &str)
{
    trace!("outsensor::set({})", data);
    let mut s = OUTSENSOR.lock().unwrap();
    *s = data.to_string();
}
// EOF
