// influxdb.rs

use super::sensordata::MyData;
use super::startup;

use anyhow::anyhow;
use chrono::*;
use log::*;
use std::{ffi::OsString, io::Write, process::*, sync::Arc, thread, time};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct InfluxSender {
    mydata: Arc<MyData>,
    interval: i64,
    binary: Option<OsString>,
    url: String,
    token: String,
    org: String,
    bucket: String,
    measurement: String,
}

impl InfluxSender {
    pub fn new(opts: &startup::OptsCommon, mydata: Arc<MyData>) -> Self {
        InfluxSender {
            mydata,
            interval: opts.send_interval,
            binary: opts.influx_binary.clone(),
            url: opts.db_url.clone(),
            token: opts.token.clone(),
            org: opts.org.clone(),
            bucket: opts.bucket.clone(),
            measurement: opts.measurement.clone(),
        }
    }

    pub fn start_db_send(self) {
        // Start a new background thread for database inserts
        thread::spawn(move || {
            self.run_db_send();
        });
    }

    fn run_db_send(self) {
        loop {
            let sender_i = self.clone();

            let jh = thread::spawn(move || match sender_i.binary {
                Some(ref _bin) => sender_i.db_send_external().unwrap(),
                None => sender_i.db_send_internal().unwrap(),
            });
            debug!(
                "InfluxDB data push thread started as id {:?}",
                jh.thread().id()
            );
            // We are blocking in join() until child thread exits -- should never happen.
            let res = jh.join();
            error!("InfluxDB thread exited! Reason: {:?}", res);
            thread::sleep(time::Duration::new(10, 0));
            error!("Restarting InfluxDB thread...");
        }
    }

    // Use the Rust native influxdb client library
    fn db_send_internal(self) -> anyhow::Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;
        let (chan_tx, mut chan_rx) = mpsc::channel::<Vec<influxdb_client::Point>>(10);

        runtime.spawn(async move {
            while let Some(points) = chan_rx.recv().await {
                debug!("influxdb data: {:?}", &points);
                let influx_client = influxdb_client::Client::new(&self.url, &self.token)
                    .with_org(&self.org)
                    .with_bucket(&self.bucket)
                    .with_precision(influxdb_client::Precision::S);

                if let Err(e) = influx_client
                    .insert_points(&points, influxdb_client::TimestampOptions::FromPoint)
                    .await
                {
                    error!("InfluxDB client error: {:?}", e);
                    break; // bail out, client is broken
                }
                info!("****** InfluxDB: inserted {} points", points.len());
            }
        });

        loop {
            let waitsec = self.interval - (Utc::now().timestamp() % self.interval);
            // wait until next interval start
            thread::sleep(time::Duration::new(waitsec as u64, 0));

            trace!("influxdb::db_send_internal() active");
            let timestamp = Utc::now().timestamp();
            let timestamp_i = timestamp - (timestamp % self.interval);

            let mut points = Vec::with_capacity(8);
            for datapoint in self.mydata.averages_db() {
                points.push(
                    influxdb_client::Point::new(&self.measurement)
                        .tag("sensor", datapoint.0.as_str())
                        .field("value", datapoint.1)
                        .timestamp(timestamp_i),
                );
            }
            if !points.is_empty() {
                runtime.block_on(chan_tx.send(points))?;
            }
        }
    }

    fn db_send_external(self) -> anyhow::Result<()> {
        let mut points = Vec::with_capacity(8);
        loop {
            let waitsec = self.interval - (Utc::now().timestamp() % self.interval);
            // wait until next interval start
            thread::sleep(time::Duration::new(waitsec as u64, 0));

            trace!("influxdb::db_send_ext() active");
            let ts = Utc::now().timestamp();
            let ts_i = ts - (ts % self.interval);

            points.clear();
            for datapoint in self.mydata.averages_db() {
                points.push(format!(
                    "{},sensor={} value={:.2} {}\n",
                    self.measurement, &datapoint.0, datapoint.1, ts_i
                ));
            }
            if !points.is_empty() {
                match self.influx_run_cmd(&points) {
                    Ok(_) => {
                        info!("****** InfluxDB: inserted {} points", points.len());
                    }
                    Err(e) => {
                        error!("InfluxDB client error: {:?}", e);
                    }
                }
            }
        }
    }

    fn influx_run_cmd(&self, points: &[String]) -> anyhow::Result<()> {
        let line_data = points.join("\n");
        let iargs = [
            "write",
            "--precision",
            "s",
            "--host",
            &self.url,
            "--token",
            &self.token,
            "--org",
            &self.org,
            "--bucket",
            &self.bucket,
        ];
        trace!("Running {:?} {}", &self.binary, iargs.join(" "));
        debug!("data:\n{}", line_data);
        let mut process = Command::new(self.binary.as_ref().unwrap())
            .args(&iargs)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pipe_in = process.stdin.as_mut().unwrap();
        pipe_in.write_all(line_data.as_bytes()).unwrap();
        let process_output = process.wait_with_output().unwrap();
        if process_output.status.success()
            && process_output.stdout.is_empty()
            && process_output.stderr.is_empty()
        {
            Ok(())
        } else {
            error!(
                "influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
                process_output.status.code().unwrap(),
                String::from_utf8_lossy(&process_output.stderr),
                String::from_utf8_lossy(&process_output.stdout)
            );
            Err(anyhow!(process_output.status))
        }
    }
}
// EOF
