// sensordata.rs

use super::startup;
use super::tbuf::{Tbuf, Tdata};

use log::*;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc, thread, time};

// Note:
// avgs_t[0] is used for returning the outside temp average
// avgs_t[1] is used for the average temp to be sent to db

type SensorData = HashMap<String, Tbuf>;

pub struct MyData {
    sensor_data: RwLock<SensorData>,
    out_sensor: RwLock<String>,
    averages_t: RwLock<Vec<u64>>,
}

pub fn start_expire(mydata: Arc<MyData>, opts: &startup::OptsCommon) {
    let interval = opts.expire_interval;
    thread::spawn(move || {
        run_expire(mydata, interval);
    });
}

fn run_expire(mydata: Arc<MyData>, interval: u64) {
    loop {
        let mydata_e = mydata.clone();
        let jh = thread::spawn(move || {
            mydata_e.expire(interval);
        });
        debug!(
            "Sensor data expire thread started as id {:?}",
            jh.thread().id()
        );
        // We are blocking in join() until child thread exits -- should never happen.
        let result = jh.join();
        error!("Expire thread exited, reason: {:?}", result);
        thread::sleep(time::Duration::new(10, 0));
        error!("Restarting expire thread...");
    }
}

#[allow(dead_code)]
impl MyData {
    pub fn new(opts: &startup::OptsCommon) -> Self {
        MyData {
            sensor_data: RwLock::new(SensorData::with_capacity(8)),
            out_sensor: RwLock::new(opts.out_sensor.clone()),
            averages_t: RwLock::new([opts.average_out_t, opts.average_db_t].to_vec()),
        }
    }

    fn expire(&self, interval: u64) {
        let wait_sec = time::Duration::new(interval, 0);
        loop {
            thread::sleep(wait_sec);
            trace!("sensordata_expire active");
            {
                for (sensorid, tbuf) in self.sensor_data.write().iter_mut() {
                    let n_expired = tbuf.expire();
                    if n_expired > 0 {
                        tbuf.update_averages();
                        info!(
                            "****** Sensor {} expired {} point{}, {} left.",
                            sensorid,
                            n_expired,
                            if n_expired > 1 { "s" } else { "" },
                            tbuf.len()
                        );
                    }
                }
            }
        }
    }

    pub fn add<S: AsRef<str>>(&self, sensorid: S, temp: f32) {
        let mut sd = self.sensor_data.write();
        if !sd.contains_key(sensorid.as_ref()) {
            sd.insert(
                sensorid.as_ref().into(),
                Tbuf::new(&*self.averages_t.read()),
            );
        }
        let tbuf = sd.get_mut(sensorid.as_ref()).unwrap();
        tbuf.add(Tdata::new(temp));
    }

    pub fn average_out_t(&self) -> u64 {
        self.averages_t.read()[0]
    }

    pub fn average_db_t(&self) -> u64 {
        self.averages_t.read()[1]
    }

    pub fn average_get<S: AsRef<str>>(&self, sensorid: S, t: u64) -> Option<f64> {
        let sd = self.sensor_data.read();
        if !sd.contains_key(sensorid.as_ref()) {
            return None;
        }
        sd.get(sensorid.as_ref()).unwrap().average(t)
    }

    pub fn average_out(&self) -> Option<f64> {
        let avg_t_out = self.average_out_t();
        let outsensor = self.out_sensor.read();
        self.average_get(&*outsensor, avg_t_out)
    }

    pub fn sensors_list(&self) -> Vec<String> {
        // Return Vec of Strings listing all the sensor ids we have
        self.sensor_data.read().keys().cloned().collect::<Vec<_>>()
    }

    pub fn averages_db(&self) -> Vec<(String, f64)> {
        let avg_t_db = self.average_db_t();
        self.sensor_data
            .read()
            .iter()
            .filter(|(_k, v)| v.len() > 3)
            .map(|(k, v)| (k.clone(), v.average(avg_t_db).unwrap()))
            .collect::<Vec<(String, f64)>>()
    }

    pub fn dump(&self) {
        // Just dump our internal sensor data into log
        let sd = self.sensor_data.read();
        debug!("dump: Have {} sensors.", sd.len());
        for (sensorid, tbuf) in sd.iter() {
            debug!("dump: Sensor {} tbuf={:?}", sensorid, tbuf);
        }
    }

    pub fn get_outsensor(&self) -> String {
        // Return our out_sensor id as cloned/owned String
        self.out_sensor.read().clone()
    }

    pub fn set_outsensor<S: AsRef<str>>(&self, data: S) {
        let mut s = self.out_sensor.write();
        *s = data.as_ref().to_string();
    }
}
// EOF
