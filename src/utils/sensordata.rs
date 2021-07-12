// utils/sensordata.rs

use super::{options, tbuf};
use log::*;
use std::{collections::HashMap, lazy::*, sync::*, thread, time};

// our global persistent state, with locking
type SensorData = HashMap<String, tbuf::Tbuf>;
static SDATA: SyncLazy<RwLock<SensorData>> =
    SyncLazy::new(|| RwLock::new(SensorData::with_capacity(8)));
static OUT_SENSOR: SyncLazy<RwLock<String>> = SyncLazy::new(|| RwLock::new(String::new()));
static AVGS_T: SyncLazy<RwLock<Vec<u64>>> = SyncLazy::new(|| RwLock::new(Vec::new()));

// Note:
// avgs_t[0] is used for returning the outside temp average
// avgs_t[1] is used for the average temp to be sent to db

pub fn init(opt: &options::GlobalServerOptions) -> thread::JoinHandle<()> {
    trace!("sensordata::init()");
    set_outsensor(&opt.out_sensor);
    // Triggering lazy initialization
    {
        let _n_sensors = SDATA.read().unwrap().len();
    }
    {
        // Saving our avgs_t
        let mut a_t = AVGS_T.write().unwrap();
        *a_t = [opt.avg_t_out, opt.avg_t_db].to_vec();
    }
    let interval = opt.expire_interval;
    thread::spawn(move || {
        sensordata_expire(interval);
    })
}

// This is run in its own thread while program is running
fn sensordata_expire(interval: u64) {
    loop {
        thread::sleep(time::Duration::from_secs(interval));
        trace!("sensordata_expire active");
        {
            for (sensorid, tbuf) in SDATA.write().unwrap().iter_mut() {
                let n_exp = tbuf.expire();
                if n_exp > 0 {
                    tbuf.update_avgs();
                    trace!("Expired: sensor {} n_exp={}", sensorid, n_exp);
                }
            }
        }
    }
}

pub fn add(sensorid: &str, temp: f32) {
    trace!("sensordata::add({}, {})", sensorid, temp);
    let mut sd = SDATA.write().unwrap();
    if !sd.contains_key(sensorid) {
        let avgs_t = AVGS_T.read().unwrap();
        sd.insert(sensorid.to_string(), tbuf::Tbuf::new(&*avgs_t));
    }
    let tbuf = sd.get_mut(sensorid).unwrap();
    tbuf.add(tbuf::Tdata::new(temp));
}

pub fn get_avg_t_out() -> u64 {
    AVGS_T.read().unwrap()[0]
}

pub fn get_avg_t_db() -> u64 {
    AVGS_T.read().unwrap()[1]
}

pub fn get_avg(sensorid: &str, t: u64) -> Option<f64> {
    trace!("sensordata::get_avg({}, {})", sensorid, t);
    let sd = SDATA.read().unwrap();
    if !sd.contains_key(sensorid) {
        return None;
    }
    sd.get(sensorid).unwrap().avg(t)
}

pub fn get_avg_out() -> Option<f64> {
    trace!("sensordata::get_avg_out()");
    get_avg(&get_outsensor(), get_avg_t_out())
}

pub fn sensors_list() -> Vec<String> {
    // Return Vec of Strings listing all the sensor ids we have, as cloned/owned strings
    let list = SDATA.read().unwrap().keys().cloned().collect::<Vec<_>>();
    trace!("sensordata::sensor_list() --> {:?}", list);
    list
}

pub fn sensors_list3() -> Vec<String> {
    // Return Vec of Strings listing all the sensor ids that have at least
    // 3 datapoints in them, as cloned/owned strings
    let sd = SDATA.read().unwrap();
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
    info!("dump: out_sensor={}", &get_outsensor());
    let sd = SDATA.read().unwrap();
    info!("dump: Have {} sensors.", sd.len());
    for (sensorid, tbuf) in sd.iter() {
        info!("dump: Sensor {} tbuf={:?}", sensorid, tbuf);
    }
}

pub fn get_outsensor() -> String {
    // Return our out_sensor id as cloned/owned String
    OUT_SENSOR.read().unwrap().clone()
}

pub fn set_outsensor(data: &str) {
    trace!("sensordata::set_outsensor({})", data);
    let mut s = OUT_SENSOR.write().unwrap();
    *s = data.to_string();
}

// EOF
