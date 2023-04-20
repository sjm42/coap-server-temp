// influxdb.rs

use super::config;
use super::sensordata::MyData;

use chrono::*;
use futures::stream;
use influxdb2::{api::write::TimestampPrecision, models::DataPoint, Client};
use log::*;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct InfluxSender {
    mydata: Arc<MyData>,
    interval: i64,
    url: String,
    token: String,
    org: String,
    bucket: String,
    measurement: String,
}

impl InfluxSender {
    pub fn new(opts: &config::OptsCommon, mydata: Arc<MyData>) -> Self {
        InfluxSender {
            mydata,
            interval: opts.send_interval,
            url: opts.db_url.clone(),
            token: opts.token.clone(),
            org: opts.org.clone(),
            bucket: opts.bucket.clone(),
            measurement: opts.measurement.clone(),
        }
    }

    pub async fn run_db_send(self) {
        loop {
            let sender_i = self.clone();
            if let Err(e) = sender_i.db_send().await {
                error!("db_send() failed: {e}");
            }
            error!("InfluxDB task exited, should not happen");
            sleep(Duration::new(10, 0)).await;
            error!("Restarting InfluxDB tasK...");
        }
    }

    // keep sending data into database
    async fn db_send(self) -> anyhow::Result<()> {
        let url = self.url.clone();
        let token = self.token.clone();
        let org = self.org.clone();
        let bucket = self.bucket.clone();

        loop {
            let waitsec = self.interval - (Utc::now().timestamp() % self.interval);
            // wait until next interval start
            sleep(Duration::new(waitsec as u64, 0)).await;

            let timestamp = Utc::now().timestamp();
            // in case we overslept :D
            let timestamp_i = timestamp - (timestamp % self.interval);

            let mut points = Vec::with_capacity(16);

            for datapoint in self.mydata.averages_db().await {
                points.push(
                    DataPoint::builder(&self.measurement)
                        .tag("sensor", datapoint.0.as_str())
                        .field("value", datapoint.1)
                        .timestamp(timestamp_i)
                        .build()?,
                );
            }

            if !points.is_empty() {
                debug!("influxdb data: {points:?}");
                let n_points = points.len();
                let influx_client = Client::new(&url, &org, &token);
                match influx_client
                    .write_with_precision(
                        &bucket,
                        stream::iter(points),
                        TimestampPrecision::Seconds,
                    )
                    .await
                {
                    Ok(_) => info!("****** InfluxDB: inserted {n_points} points"),
                    Err(e) => error!("InfluxDB client error: {e:?}"),
                }
            }
        }
    }
}

// EOF
