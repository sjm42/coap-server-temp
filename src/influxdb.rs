// influxdb.rs

use super::sensordata;
use super::startup;

use anyhow::anyhow;
use chrono::*;
use log::*;
use std::{ffi::OsString, io::Write, process::*, sync::Arc, thread, time};
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct InfluxCtx {
    md: Arc<sensordata::MyData>,
    interval: i64,
    binary: Option<OsString>,
    url: String,
    token: String,
    org: String,
    bucket: String,
    measurement: String,
}

impl InfluxCtx {
    pub fn new(opts: &startup::OptsCommon, md: Arc<sensordata::MyData>) -> Self {
        InfluxCtx {
            md,
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
            let ctx_i = self.clone();

            let jh = thread::spawn(move || match ctx_i.binary {
                Some(ref _bin) => ctx_i.db_send_external(),
                None => ctx_i.db_send_internal(),
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
    fn db_send_internal(self) {
        let rt = Runtime::new().unwrap();
        loop {
            let waitsec = self.interval - (Utc::now().timestamp() % self.interval);
            // wait until next interval start
            thread::sleep(time::Duration::new(waitsec as u64, 0));

            trace!("influxdb::db_send_internal() active");
            let ts = Utc::now().timestamp();
            let ts_i = ts - (ts % self.interval);

            let mut pts = Vec::with_capacity(8);
            for datapoint in self.md.sensors_db() {
                pts.push(
                    influxdb_client::Point::new(&self.measurement)
                        .tag("sensor", datapoint.0.as_str())
                        .field("value", datapoint.1)
                        .timestamp(ts_i),
                );
            }
            if !pts.is_empty() {
                let c = influxdb_client::Client::new(&self.url, &self.token)
                    .with_org(&self.org)
                    .with_bucket(&self.bucket)
                    .with_precision(influxdb_client::Precision::S);

                rt.spawn(async move {
                    // ownership of pts and c are moved into here
                    // hence, new ones must be created each time before calling this
                    debug!("influxdb data: {:?}", &pts);
                    match c
                        .insert_points(&pts, influxdb_client::TimestampOptions::FromPoint)
                        .await
                    {
                        Ok(_) => {
                            info!("****** InfluxDB: inserted {} points", pts.len());
                        }
                        Err(e) => {
                            error!("InfluxDB client error: {:?}", e);
                        }
                    }
                });
            }
        }
    }

    fn db_send_external(self) {
        let mut data_points = Vec::with_capacity(8);
        loop {
            let waitsec = self.interval - (Utc::now().timestamp() % self.interval);
            // wait until next interval start
            thread::sleep(time::Duration::new(waitsec as u64, 0));

            trace!("influxdb::db_send_ext() active");
            let ts = Utc::now().timestamp();
            let ts_i = ts - (ts % self.interval);

            data_points.clear();
            for datapoint in self.md.sensors_db() {
                data_points.push(format!(
                    "{},sensor={} value={:.2} {}\n",
                    self.measurement, &datapoint.0, datapoint.1, ts_i
                ));
            }
            if !data_points.is_empty() {
                match self.influx_run_cmd(&data_points) {
                    Ok(_) => {
                        info!("****** InfluxDB: inserted {} points", data_points.len());
                    }
                    Err(e) => {
                        error!("InfluxDB client error: {:?}", e);
                    }
                }
            }
        }
    }

    fn influx_run_cmd(&self, data_points: &[String]) -> anyhow::Result<()> {
        let line_data = data_points.join("\n");
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
        let mut p = Command::new(self.binary.as_ref().unwrap())
            .args(&iargs)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let p_in = p.stdin.as_mut().unwrap();
        p_in.write_all(line_data.as_bytes()).unwrap();
        let out = p.wait_with_output().unwrap();
        if out.status.success() && out.stdout.is_empty() && out.stderr.is_empty() {
            Ok(())
        } else {
            error!(
                "influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
                out.status.code().unwrap(),
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );
            Err(anyhow!(out.status))
        }
    }
}
// EOF
