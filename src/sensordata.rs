// sensordata.rs

use super::config;
use super::tbuf::{Tbuf, Tdata};

use log::*;
use std::{collections::HashMap, sync::Arc, time};
use tokio::sync::RwLock;

// Note:
// avgs_t[0] is used for returning the outside temp average
// avgs_t[1] is used for the average temp to be sent to db

type SensorData = HashMap<String, Tbuf>;

#[derive(Default)]
pub struct MyData {
    sensor_data: RwLock<SensorData>,
    out_sensor: RwLock<String>,
    averages_t: RwLock<Vec<u64>>,
}

pub async fn run_expire(mydata: Arc<MyData>, interval: u64) {
    loop {
        mydata.expire(interval).await;
        error!("Expire task exited, should not happen");
        tokio::time::sleep(time::Duration::new(10, 0)).await;
        error!("Restarting expire task...");
    }
}

#[allow(dead_code)]
impl MyData {
    pub fn new(opts: &config::OptsCommon) -> Self {
        MyData {
            sensor_data: RwLock::new(SensorData::with_capacity(8)),
            out_sensor: RwLock::new(opts.out_sensor.clone()),
            averages_t: RwLock::new(vec![opts.average_out_t, opts.average_db_t]),
        }
    }

    async fn expire(&self, interval: u64) {
        let wait_duration = time::Duration::new(interval, 0);
        loop {
            tokio::time::sleep(wait_duration).await;
            trace!("sensordata_expire active");

            for (sensorid, tbuf) in self.sensor_data.write().await.iter_mut() {
                let n_expired = tbuf.expire();
                if n_expired > 0 {
                    tbuf.update_averages();
                    info!(
                        "****** Sensor {sensorid} expired {n_expired} point{}, {} left.",
                        if n_expired > 1 { "s" } else { "" },
                        tbuf.len()
                    );
                }
            }
        }
    }

    pub async fn add<S: AsRef<str>>(&self, sensor_id: S, temp: f32) {
        let mut sensor_data = self.sensor_data.write().await;

        if !sensor_data.contains_key(sensor_id.as_ref()) {
            sensor_data.insert(
                sensor_id.as_ref().into(),
                Tbuf::new(&self.averages_t.read().await),
            );
        }
        match sensor_data.get_mut(sensor_id.as_ref()) {
            Some(tbuf) => {
                tbuf.add(Tdata::new(temp));
            }
            None => {
                error!("What? Tbuf is gone.");
            }
        }
    }

    pub async fn average_out_t(&self) -> u64 {
        self.averages_t.read().await[0]
    }

    pub async fn average_db_t(&self) -> u64 {
        self.averages_t.read().await[1]
    }

    pub async fn average_get<S: AsRef<str>>(&self, sensor_id: S, t: u64) -> Option<f64> {
        let sensor_data = self.sensor_data.read().await;
        match sensor_data.get(sensor_id.as_ref()) {
            None => None,
            Some(d) => d.average(t),
        }
    }

    // out_sensor may have a comma-separated list of sensor ids
    pub async fn average_out(&self) -> Option<f64> {
        let out_sensor = self.out_sensor.read().await.clone();
        let out_t = self.average_out_t().await;
        for s in out_sensor.split(',') {
            if let Some(f) = self.average_get(s, out_t).await {
                return Some(f);
            }
        }
        None
    }

    // Return Vec of Strings listing all the sensor ids we have
    pub async fn sensors_list(&self) -> Vec<String> {
        self.sensor_data.read().await.keys().cloned().collect()
    }

    pub async fn averages_db(&self) -> Vec<(String, f64)> {
        let avg_t_db = self.average_db_t().await;
        self.sensor_data
            .read()
            .await
            .iter()
            .filter(|(_k, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), v.average(avg_t_db).unwrap_or(0.0)))
            .collect()
    }

    // Just dump our internal sensor data into log
    pub async fn dump(&self) {
        let sensor_data = self.sensor_data.read().await;
        debug!("dump: Have {} sensors.", sensor_data.len());
        for (sensor_id, tbuf) in sensor_data.iter() {
            debug!("dump: Sensor {sensor_id} tbuf={tbuf:?}");
        }
    }

    // Return our out_sensor id
    pub async fn get_outsensor(&self) -> String {
        self.out_sensor.read().await.clone()
    }

    pub async fn set_outsensor<S: AsRef<str>>(&self, data: S) {
        let mut s = self.out_sensor.write().await;
        *s = data.as_ref().to_string();
    }
}
// EOF
