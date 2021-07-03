// utils/sensordata.rs

use log::*;
use std::collections::HashMap;
use std::lazy::*;
use std::sync::*;
use std::thread;
use std::time;

use crate::utils::tbuf;

// our global persistent state, with locking
type SensorData = HashMap<String, tbuf::Tbuf>;
static SDATA: SyncLazy<Mutex<SensorData>> = SyncLazy::new(|| Mutex::new(SensorData::new()));
static OUTSENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));

// my outside temperature sensor id, for convenience
// can be overridden any time using outsensor_set()
const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";

// use 5min averages for time-series db
pub const AVG_T_TDB: u64 = 60 * 5;
// use 15min averages for outside temperature queries
pub const AVG_T_OUT: u64 = 60 * 15;

pub fn init() {
    info!("sensordata::init()");
    outsensor_set(DEFAULT_OUTSENSOR);
    // Triggering lazy initialization
    {
        let _n_sensors = SDATA.lock().unwrap().len();
    }
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
                    tbuf.update_avgs();
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
        sd.insert(
            sensorid.to_string(),
            tbuf::Tbuf::new(&[AVG_T_TDB, AVG_T_OUT]),
        );
    }
    let tbuf = sd.get_mut(sensorid).unwrap();
    tbuf.add(tbuf::Tdata::new(temp));
}

pub fn get_avg(sensorid: &str, t: u64) -> Option<f64> {
    trace!("sensordata::get_avg({}, {})", sensorid, t);
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(sensorid) {
        return None;
    }
    sd.get(sensorid).unwrap().avg(t)
}

pub fn sensor_list() -> Vec<String> {
    let list = SDATA.lock().unwrap().keys().cloned().collect::<Vec<_>>();
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
    // trace!("outsensor::get() --> {:?}", &s);
    s.clone()
}

pub fn outsensor_set(data: &str) {
    trace!("outsensor::set({})", data);
    let mut s = OUTSENSOR.lock().unwrap();
    *s = data.to_string();
}

// EOF
