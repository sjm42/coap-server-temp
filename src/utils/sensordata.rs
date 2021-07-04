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
static OUT_SENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));
static AVGS_T: SyncLazy<Mutex<Vec<u64>>> = SyncLazy::new(|| Mutex::new(Vec::new()));

// Note:
// avgs_t[0] is used for returning the outside temp average
// avgs_t[1] is used for the average temp to be sent to db
pub fn init(out_sensor: &str, avgs_t: &[u64]) {
    info!("sensordata::init()");
    if avgs_t.len() != 2 {
        panic!("Must have exactly two avgs");
    }
    set_outsensor(out_sensor);
    // Triggering lazy initializations
    {
        let _n_sensors = SDATA.lock().unwrap().len();
        let mut a_t = AVGS_T.lock().unwrap();
        *a_t = avgs_t.to_vec();
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
        let avgs_t = AVGS_T.lock().unwrap();
        sd.insert(sensorid.to_string(), tbuf::Tbuf::new(&*avgs_t));
    }
    let tbuf = sd.get_mut(sensorid).unwrap();
    tbuf.add(tbuf::Tdata::new(temp));
}

pub fn get_avg_t_out() -> u64 {
    let avgs_t = AVGS_T.lock().unwrap();
    avgs_t[0]
}

pub fn get_avg_t_db() -> u64 {
    let avgs_t = AVGS_T.lock().unwrap();
    avgs_t[1]
}

pub fn get_avg(sensorid: &str, t: u64) -> Option<f64> {
    trace!("sensordata::get_avg({}, {})", sensorid, t);
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(sensorid) {
        return None;
    }
    sd.get(sensorid).unwrap().avg(t)
}

pub fn get_avg_out() -> Option<f64> {
    trace!("sensordata::get_avg_out()");
    get_avg(&get_outsensor(), get_avg_t_out())
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

pub fn get_outsensor() -> String {
    let s = OUT_SENSOR.lock().unwrap();
    s.clone()
}

pub fn set_outsensor(data: &str) {
    trace!("sensordata::set_outsensor({})", data);
    let mut s = OUT_SENSOR.lock().unwrap();
    *s = data.to_string();
}

// EOF
