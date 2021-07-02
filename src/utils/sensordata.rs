// utils/sensordata.rs

use log::*;
use std::collections::HashMap;
use std::lazy::*;
use std::sync::*;
use std::thread;
use std::time;

use crate::utils::tbuf;

type SensorData = HashMap<String, tbuf::Tbuf>;
static SDATA: SyncLazy<Mutex<SensorData>> = SyncLazy::new(|| Mutex::new(SensorData::new()));

const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";
static OUTSENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));

pub fn init() {
    info!("sensordata::init()");
    outsensor_set(DEFAULT_OUTSENSOR);
    // Triggering lazy initialization
    let _n_sensors = SDATA.lock().unwrap().len();
    let _thr_expire = thread::spawn(|| {
        sensordata_expire();
    });
}

// This is run in its own thread while program is running
fn sensordata_expire() {
    loop {
        trace!("sensordata_expire active");
        {
            let mut sd = SDATA.lock().unwrap();
            for (_sensorid, tbuf) in sd.iter_mut() {
                let len1 = tbuf.len();
                if tbuf.expire() {
                    tbuf.upd_avg();
                }
                let n_exp = len1 - tbuf.len();
                if n_exp > 0 {
                    trace!("Expired: sensor {} n_exp={}", _sensorid, n_exp);
                }
            }
        }
        thread::sleep(time::Duration::from_secs(30));
    }
}

pub fn add(sensorid: &str, temp: f32) {
    trace!("sensordata::add({}, {})", sensorid, temp);
    let mut sd = SDATA.lock().unwrap();
    if !sd.contains_key(sensorid) {
        let new_tbuf = tbuf::Tbuf::new();
        sd.insert(sensorid.to_string(), new_tbuf);
    }
    let tbuf = sd.get_mut(sensorid).unwrap();
    tbuf.add(tbuf::Tdata::new(temp));
}

pub fn get_avg5(sensorid: &str) -> Option<f32> {
    trace!("sensordata::get_avg5({})", sensorid);
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(sensorid) {
        return None;
    }
    return Some(sd.get(sensorid).unwrap().avg5());
}

pub fn get_avg15(sensorid: &str) -> Option<f32> {
    trace!("sensordata::get_avg15({})", sensorid);
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(sensorid) {
        return None;
    }
    return Some(sd.get(sensorid).unwrap().avg15());
}

pub fn sensor_list() -> Vec<String> {
    let sd = SDATA.lock().unwrap();
    let list = sd.keys().cloned().collect::<Vec<_>>();
    trace!("sensordata::sensor_list() --> {:?}", list);
    list
}

pub fn sensor_list3() -> Vec<String> {
    let sd = SDATA.lock().unwrap();
    let list = sd
        .keys()
        .filter(|s| sd.get(*s).unwrap().len() >= 3)
        .cloned()
        .collect::<Vec<_>>();
    trace!("sensordata::sensor_list3() --> {:?}", list);
    list
}

pub fn dump() {
    let sd = SDATA.lock().unwrap();
    info!("dump: {} sensors.", sd.len());
    for (sensorid, tbuf) in sd.iter() {
        info!("dump: sensor {} tbuf={:?}", sensorid, tbuf);
    }
}

pub fn outsensor_get() -> String {
    let s = OUTSENSOR.lock().unwrap();
    // trace!("outsensor::get() --> {:?}", s);
    s.to_string()
}

pub fn outsensor_set(data: &str) {
    trace!("outsensor::set({})", data);
    let mut s = OUTSENSOR.lock().unwrap();
    *s = data.to_string();
}

// EOF
