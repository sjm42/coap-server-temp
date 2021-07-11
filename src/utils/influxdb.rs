// utils/influxdb.rs

use super::{options, sensordata};
use chrono::*;
use influxdb_client;
use log::*;
use std::{io::Write, path::Path, process::*, thread, time};
use tokio::runtime::Runtime;

pub fn init(opt: &options::GlobalServerOptions) -> thread::JoinHandle<()> {
    trace!("influxdb::init()");

    let interval = opt.send_interval;
    let binary = opt.influx_binary.clone();
    let url = opt.db_url.clone();
    let token = opt.token.clone();
    let org = opt.org.clone();
    let bucket = opt.bucket.clone();
    let measurement = opt.measurement.clone();
    // Start a new background thread for database inserts
    match binary {
        None => {
            info!("Using the internal InfluxDB client");
            thread::spawn(move || {
                db_send_internal(interval, &url, &token, &org, &bucket, &measurement)
            })
        }
        Some(fbin) => {
            info!("Using external Influx binary {:?}", fbin);
            thread::spawn(move || {
                db_send_external(interval, &fbin, &url, &token, &org, &bucket, &measurement)
            })
        }
    }
}

// Use the Rust native influxdb client library
fn db_send_internal(
    interval: i64,
    url: &str,
    token: &str,
    org: &str,
    bucket: &str,
    measurement: &str,
) {
    let rt = Runtime::new().unwrap();
    loop {
        let waitsec = interval - (Utc::now().timestamp() % interval);
        // wait until next interval start
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_internal() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % interval);

        let mut pts = Vec::with_capacity(8);
        for sensorid in sensordata::sensors_list3() {
            pts.push(
                influxdb_client::Point::new(measurement)
                    .tag("sensor", sensorid.as_str())
                    .field(
                        "value",
                        sensordata::get_avg(&sensorid, sensordata::get_avg_t_db()).unwrap(),
                    )
                    .timestamp(ts_i),
            );
        }
        if !pts.is_empty() {
            let c = influxdb_client::Client::new(url, token)
                .with_org(org)
                .with_bucket(bucket)
                .with_precision(influxdb_client::Precision::S);

            rt.spawn(async move {
                // ownership of pts and c are moved into here
                // hence, new ones must be created each time before calling this
                trace!("influxdb data: {:?}", &pts);
                match c
                    .insert_points(&pts, influxdb_client::TimestampOptions::FromPoint)
                    .await
                {
                    Ok(_) => {
                        info!("****** influxdb: inserted {} points", pts.len());
                    }
                    Err(e) => {
                        error!("InfluxDB client error: {:?}", e);
                    }
                }
            });
        }
    }
}

fn db_send_external(
    interval: i64,
    bin: &Path,
    url: &str,
    token: &str,
    org: &str,
    bucket: &str,
    measurement: &str,
) {
    let mut data_points = Vec::with_capacity(8);
    loop {
        let waitsec = interval - (Utc::now().timestamp() % interval);
        // wait until next interval start
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_ext() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % interval);

        data_points.clear();
        for sensorid in sensordata::sensors_list3() {
            data_points.push(format!(
                "{},sensor={} value={:.2} {}\n",
                measurement,
                &sensorid,
                sensordata::get_avg(&sensorid, sensordata::get_avg_t_db()).unwrap(),
                ts_i
            ));
        }
        // Only send if we have anything to send...
        if !data_points.is_empty() {
            match influx_run_cmd(&data_points, bin, url, token, org, bucket) {
                Ok(_) => {
                    info!("****** influxdb: inserted {} points", data_points.len());
                }
                Err(e) => {
                    error!("InfluxDB client error: {:?}", e);
                }
            }
        }
    }
}

// Run the external influx command to write data.
fn influx_run_cmd(
    data_points: &[String],
    bin: &Path,
    url: &str,
    token: &str,
    org: &str,
    bucket: &str,
) -> Result<(), ExitStatus> {
    let line_data = data_points.join("\n");
    let iargs = [
        "write",
        "--precision",
        "s",
        "--host",
        url,
        "--token",
        token,
        "--org",
        org,
        "--bucket",
        bucket,
    ];
    trace!("Running {:?} {}", bin, iargs.join(" "));
    trace!("data:\n{}", line_data);
    let mut p = Command::new(bin)
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
            String::from_utf8(out.stderr).unwrap(),
            String::from_utf8(out.stdout).unwrap()
        );
        Err(out.status)
    }
}
// EOF
