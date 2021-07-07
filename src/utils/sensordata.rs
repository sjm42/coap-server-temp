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
    trace!("sensordata::init()");
    assert!(avgs_t.len() == 2, "Must have exactly two avgs_t: for outside temp and db inserts.");

    let _thr_expire = thread::spawn(|| {
        sensordata_expire();
    });
    set_outsensor(out_sensor);
    // Triggering lazy initialization
    let _n_sensors = SDATA.lock().unwrap().len();
    // Saving our avgs_t
    let mut a_t = AVGS_T.lock().unwrap();
    *a_t = avgs_t.to_vec();
}

// This is run in its own thread while program is running
fn sensordata_expire() {
    loop {
        thread::sleep(time::Duration::from_secs(30));
        trace!("sensordata_expire active");
        {
            for (sensorid, tbuf) in SDATA.lock().unwrap().iter_mut() {
                let len1 = tbuf.len();
                // expire() method returns true if changes were done
                // i.e. some data was expired
                if tbuf.expire() {
                    tbuf.update_avgs();
                }
                let n_exp = len1 - tbuf.len();
                if n_exp > 0 {
                    trace!("Expired: sensor {} n_exp={}", sensorid, n_exp);
                }
            }
        }
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
    AVGS_T.lock().unwrap()[0]
}

pub fn get_avg_t_db() -> u64 {
    AVGS_T.lock().unwrap()[1]
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
    // Return Vec of Strings listing all the sensor ids we have, as cloned/owned strings
    let list = SDATA.lock().unwrap().keys().cloned().collect::<Vec<_>>();
    trace!("sensordata::sensor_list() --> {:?}", list);
    list
}

pub fn sensor_list3() -> Vec<String> {
    // Return Vec of Strings listing all the sensor ids that have at least
    // 3 datapoints in them, as cloned/owned strings
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
    // Just dump our internal sensor data into log
    {
        let s = OUT_SENSOR.lock().unwrap();
        info!("dump: out_sensor={}", *s);
    }
    let sd = SDATA.lock().unwrap();
    info!("dump: Have {} sensors.", sd.len());
    for (sensorid, tbuf) in sd.iter() {
        info!("dump: Sensor {} tbuf={:?}", sensorid, tbuf);
    }
}

pub fn get_outsensor() -> String {
    // Return our out_sensor id as cloned/owned String
    OUT_SENSOR.lock().unwrap().clone()
}

pub fn set_outsensor(data: &str) {
    trace!("sensordata::set_outsensor({})", data);
    let mut s = OUT_SENSOR.lock().unwrap();
    *s = data.to_string();
}

// EOF
