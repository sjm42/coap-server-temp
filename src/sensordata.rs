// sensordata.rs

use super::startup;
use super::tbuf;

use log::*;
use parking_lot::*;
use std::{collections::HashMap, sync::Arc, thread, time};

// Note:
// avgs_t[0] is used for returning the outside temp average
// avgs_t[1] is used for the average temp to be sent to db

type SensorData = HashMap<String, tbuf::Tbuf>;

pub struct MyData {
    sd: RwLock<SensorData>,
    out_sensor: RwLock<String>,
    avgs_t: RwLock<Vec<u64>>,
}

#[allow(dead_code)]
impl MyData {
    pub fn new(opts: &startup::OptsCommon) -> Self {
        MyData {
            sd: RwLock::new(SensorData::with_capacity(8)),
            out_sensor: RwLock::new(opts.out_sensor.clone()),
            avgs_t: RwLock::new([opts.avg_t_out, opts.avg_t_db].to_vec()),
        }
    }

    pub fn start_expire(md: Arc<Self>, opts: &startup::OptsCommon) {
        let interval = opts.expire_interval;
        thread::spawn(move || {
            Self::run_expire(md.clone(), interval);
        });
    }

    fn run_expire(md: Arc<Self>, interval: u64) {
        loop {
            let md_e = md.clone();
            let jh = thread::spawn(move || {
                md_e.expire(interval);
            });
            debug!(
                "Sensor data expire thread started as id {:?}",
                jh.thread().id()
            );
            // We are blocking in join() until child thread exits -- should never happen.
            let res = jh.join();
            error!("Expire thread exited, reason: {:?}", res);
            thread::sleep(time::Duration::new(10, 0));
            error!("Restarting expire thread...");
        }
    }

    fn expire(&self, interval: u64) {
        let delay = time::Duration::new(interval, 0);
        loop {
            thread::sleep(delay);
            trace!("sensordata_expire active");
            {
                for (sensorid, tbuf) in self.sd.write().iter_mut() {
                    let n_exp = tbuf.expire();
                    if n_exp > 0 {
                        tbuf.update_avgs();
                        info!(
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

    pub fn add<S: AsRef<str>>(&self, sensorid: S, temp: f32) {
        let mut sd = self.sd.write();
        if !sd.contains_key(sensorid.as_ref()) {
            sd.insert(
                sensorid.as_ref().into(),
                tbuf::Tbuf::new(&*self.avgs_t.read()),
            );
        }
        let tbuf = sd.get_mut(sensorid.as_ref()).unwrap();
        tbuf.add(tbuf::Tdata::new(temp));
    }

    pub fn get_avg_t_out(&self) -> u64 {
        self.avgs_t.read()[0]
    }

    pub fn get_avg_t_db(&self) -> u64 {
        self.avgs_t.read()[1]
    }

    pub fn get_avg<S: AsRef<str>>(&self, sensorid: S, t: u64) -> Option<f64> {
        let sd = self.sd.read();
        if !sd.contains_key(sensorid.as_ref()) {
            return None;
        }
        sd.get(sensorid.as_ref()).unwrap().avg(t)
    }

    pub fn get_avg_out(&self) -> Option<f64> {
        let avg_t_out = self.get_avg_t_out();
        let outsensor = self.out_sensor.read();
        self.get_avg(&*outsensor, avg_t_out)
    }

    pub fn sensors_list(&self) -> Vec<String> {
        // Return Vec of Strings listing all the sensor ids we have, as cloned/owned strings
        self.sd.read().keys().cloned().collect::<Vec<_>>()
    }

    pub fn sensors_db(&self) -> Vec<(String, f64)> {
        let avg_t_db = self.get_avg_t_db();
        self.sd
            .read()
            .iter()
            .filter(|(_k, v)| v.len() > 3)
            .map(|(k, v)| (k.clone(), v.avg(avg_t_db).unwrap()))
            .collect::<Vec<(String, f64)>>()
    }

    pub fn dump(&self) {
        // Just dump our internal sensor data into log
        let sd = self.sd.read();
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
