// utils/sensordata.rs

use super::{options, tbuf};

use log::*;
use parking_lot::*;
use std::{collections::HashMap, lazy::*, thread, time};

// our global persistent state, with locking
type SensorData = HashMap<String, tbuf::Tbuf>;
static SENSOR_DATA: SyncLazy<RwLock<SensorData>> =
    SyncLazy::new(|| RwLock::new(SensorData::with_capacity(8)));
static OUT_SENSOR: SyncLazy<RwLock<String>> = SyncLazy::new(|| RwLock::new(String::new()));
static AVERAGES_T: SyncLazy<RwLock<Vec<u64>>> = SyncLazy::new(|| RwLock::new(Vec::new()));

// Note:
// avgs_t[0] is used for returning the outside temp average
// avgs_t[1] is used for the average temp to be sent to db
pub fn init(opt: &options::GlobalServerOptions) -> thread::JoinHandle<()> {
    trace!("sensordata::init()");
    set_outsensor(&opt.out_sensor);
    // Triggering lazy initialization
    {
        let _n_sensors = SENSOR_DATA.read().len();
    }
    {
        // Saving our avgs_t
        let mut a_t = AVERAGES_T.write();
        *a_t = [opt.avg_t_out, opt.avg_t_db].to_vec();
    }
    let interval = opt.expire_interval;
    thread::spawn(move || {
        run_sensordata_expire(interval);
    })
}

// This is run in its own thread while program is running
fn run_sensordata_expire(interval: u64) {
    loop {
        let jh = thread::spawn(move || {
            sensordata_expire(interval);
        });
        debug!(
            "Sensor data expire thread started as id {:?}",
            jh.thread().id()
        );
        // We are blocking in join() until child thread exits -- should never happen.
        let res = jh.join();
        error!("Expire thread exited, reason: {:?}", res);
        thread::sleep(time::Duration::from_secs(10));
        error!("Restarting expire thread...");
    }
}

fn sensordata_expire(interval: u64) {
    let delay = time::Duration::from_secs(interval);
    loop {
        thread::sleep(delay);
        trace!("sensordata_expire active");
        {
            for (sensorid, tbuf) in SENSOR_DATA.write().iter_mut() {
                let n_exp = tbuf.expire();
                if n_exp > 0 {
                    tbuf.update_avgs();
                    debug!(
                        "****** Sensor {} expired {} point{}, {} left.",
                        sensorid,
                        n_exp,
                        if n_exp > 1 { "s" } else { "" },
                        tbuf.len()
                    );
                }
            }
        }
    }
}

pub fn add(sensorid: &str, temp: f32) {
    trace!("sensordata::add({}, {})", sensorid, temp);
    let mut sd = SENSOR_DATA.write();
    if !sd.contains_key(sensorid) {
        let avgs_t = AVERAGES_T.read();
        sd.insert(sensorid.to_string(), tbuf::Tbuf::new(&*avgs_t));
    }
    let tbuf = sd.get_mut(sensorid).unwrap();
    tbuf.add(tbuf::Tdata::new(temp));
}

pub fn get_avg_t_out() -> u64 {
    AVERAGES_T.read()[0]
}

pub fn get_avg_t_db() -> u64 {
    AVERAGES_T.read()[1]
}

pub fn get_avg(sensorid: &str, t: u64) -> Option<f64> {
    trace!("sensordata::get_avg({}, {})", sensorid, t);
    let sd = SENSOR_DATA.read();
    if !sd.contains_key(sensorid) {
        return None;
    }
    sd.get(sensorid).unwrap().avg(t)
}

pub fn get_avg_out() -> Option<f64> {
    trace!("sensordata::get_avg_out()");
    let avg_t_out = get_avg_t_out();
    let outsensor = OUT_SENSOR.read();
    get_avg(&outsensor, avg_t_out)
}

pub fn sensors_list() -> Vec<String> {
    // Return Vec of Strings listing all the sensor ids we have, as cloned/owned strings
    let list = SENSOR_DATA.read().keys().cloned().collect::<Vec<_>>();
    trace!("sensordata::sensor_list() --> {:?}", list);
    list
}

pub fn sensors_db() -> Vec<(String, f64)> {
    let avg_t_db = get_avg_t_db();
    let mut dump = vec![];
    let sd = SENSOR_DATA.read();
    for sensorid in sd.keys() {
        if sd.get(sensorid).unwrap().len() >= 3 {
            dump.push((
                sensorid.clone(),
                sd.get(sensorid).unwrap().avg(avg_t_db).unwrap(),
            ));
        }
    }
    dump
}

pub fn dump() {
    // Just dump our internal sensor data into log
    debug!("dump: out_sensor={}", &get_outsensor());
    let sd = SENSOR_DATA.read();
    debug!("dump: Have {} sensors.", sd.len());
    for (sensorid, tbuf) in sd.iter() {
        debug!("dump: Sensor {} tbuf={:?}", sensorid, tbuf);
    }
}

pub fn get_outsensor() -> String {
    // Return our out_sensor id as cloned/owned String
    OUT_SENSOR.read().clone()
}

pub fn set_outsensor(data: &str) {
    trace!("sensordata::set_outsensor({})", data);
    let mut s = OUT_SENSOR.write();
    *s = data.to_string();
}
// EOF
