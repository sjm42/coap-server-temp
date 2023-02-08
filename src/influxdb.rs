// influxdb.rs

use super::sensordata::MyData;
use super::config;

use anyhow::anyhow;
use chrono::*;
use log::*;
use std::{ffi::OsString, io::Write, process::*, sync::Arc, thread, time};
use tokio::{runtime::Runtime, sync::mpsc};

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
    pub fn new(opts: &config::OptsCommon, mydata: Arc<MyData>) -> Self {
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

    // Start a new background thread for database inserts
    pub fn start_db_send(self) {
        thread::spawn(move || {
            self.run_db_send();
        });
    }

    fn run_db_send(self) {
        loop {
            let sender_i = self.clone();
            let jh = thread::spawn(move || sender_i.db_send());
            info!(
                "InfluxDB data push thread started as id {:?}",
                jh.thread().id()
            );

            // We are blocking in join() until child thread exits -- should never happen.
            let res = jh.join();
            error!("InfluxDB thread exited! Reason: {res:?}");
            thread::sleep(time::Duration::new(10, 0));
            error!("Restarting InfluxDB thread...");
        }
    }

    // keep sending data into database
    fn db_send(self) -> anyhow::Result<()> {
        // Use the native influxdb client library?
        let internal = self.binary.is_none();

        let mut runtime: Option<Runtime> = None;
        let mut chan_tx = None;

        if internal {
            // Create a tokio runtime and a communication channel for data
            runtime = Some(tokio::runtime::Runtime::new()?);
            let (tx, mut rx) = mpsc::channel::<Vec<influxdb_client::Point>>(10);
            chan_tx = Some(tx);

            // Spawn a new async task that consumes and saves datapoints via the channel
            let url = self.url.clone();
            let token = self.token.clone();
            let org = self.org.clone();
            let bucket = self.bucket.clone();
            runtime
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("runtime gone"))?
                .spawn(async move {
                    while let Some(points) = rx.recv().await {
                        debug!("influxdb data: {points:?}");

                        // We are creating a new client each time -- once per minute, not a performance problem
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
                        drop(influx_client);
                    }
                });
        }

        loop {
            let waitsec = self.interval - (Utc::now().timestamp() % self.interval);
            // wait until next interval start
            thread::sleep(time::Duration::new(waitsec as u64, 0));

            let timestamp = Utc::now().timestamp();
            // in case we overslept :D
            let timestamp_i = timestamp - (timestamp % self.interval);

            let mut points_i = Vec::with_capacity(16);
            let mut points_e = Vec::with_capacity(16);

            for datapoint in self.mydata.averages_db() {
                if internal {
                    points_i.push(
                        influxdb_client::Point::new(&self.measurement)
                            .tag("sensor", datapoint.0.as_str())
                            .field("value", datapoint.1)
                            .timestamp(timestamp_i),
                    );
                } else {
                    points_e.push(format!(
                        "{},sensor={} value={:.2} {timestamp_i}\n",
                        self.measurement, &datapoint.0, datapoint.1,
                    ));
                }
            }

            if !points_i.is_empty() {
                if internal {
                    runtime
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("runtime gone"))?
                        .block_on(
                            chan_tx
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("chan_tx gone"))?
                                .send(points_i),
                        )?;
                } else {
                    match self.influx_run_cmd(&points_e) {
                        Ok(_) => info!("****** InfluxDB: inserted {} points", points_e.len()),
                        Err(e) => error!("InfluxDB client error: {e:?}"),
                    }
                }
            }
        }
    }

    // Spawn the external InfluxDB client and handle I/O
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
        debug!("data:\n{line_data}");

        let mut process = Command::new(
            self.binary
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("binary gone"))?,
        )
        .args(iargs)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
        let pipe_in = process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("stdin gone"))?;
        pipe_in.write_all(line_data.as_bytes())?;

        let process_output = process.wait_with_output()?;
        if process_output.status.success()
            && process_output.stdout.is_empty()
            && process_output.stderr.is_empty()
        {
            Ok(())
        } else {
            error!(
                "influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
                process_output.status.code().unwrap_or(0),
                String::from_utf8_lossy(&process_output.stderr),
                String::from_utf8_lossy(&process_output.stdout)
            );
            Err(anyhow!(process_output.status))
        }
    }
}
// EOF
