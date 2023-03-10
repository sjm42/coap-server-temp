// influxdb.rs

use super::config;
use super::sensordata::MyData;

use chrono::*;
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
                    influxdb_client::Point::new(&self.measurement)
                        .tag("sensor", datapoint.0.as_str())
                        .field("value", datapoint.1)
                        .timestamp(timestamp_i),
                );
            }

            if !points.is_empty() {
                debug!("influxdb data: {points:?}");

                // We are creating a new client each time
                // -- usually once per minute, not a performance problem
                let influx_client = influxdb_client::Client::new(&url, &token)
                    .with_org(&org)
                    .with_bucket(&bucket)
                    .with_precision(influxdb_client::Precision::S);

                match influx_client
                    .insert_points(&points, influxdb_client::TimestampOptions::FromPoint)
                    .await
                {
                    Ok(_) => info!("****** InfluxDB: inserted {} points", points.len()),
                    Err(e) => error!("InfluxDB client error: {e:?}"),
                }
            }
        }
    }
}
// EOF
