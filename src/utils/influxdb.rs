// utils/influxdb.rs

use log::*;
use std::collections::HashMap;
use std::io::Write;
use std::process::*;
use std::{thread, time};

use chrono::*;
use influxdb_client;
use tokio::runtime::Runtime;

use crate::utils::options;
use crate::utils::sensordata;

type InfOpt = HashMap<&'static str, String>;

pub fn init(interval: i64, opt: &options::CoapServerOpts) {
    trace!("influxdb::init()");
    let mut iopt: InfOpt = HashMap::new();
    iopt.insert("binary", opt.influx_binary.clone());
    iopt.insert("url", opt.influxdb_url.clone());
    iopt.insert("token", opt.influxdb_token.clone());
    iopt.insert("org", opt.influxdb_org.clone());
    iopt.insert("bucket", opt.influxdb_bucket.clone());
    iopt.insert("measurement", opt.influxdb_measurement.clone());

    // Start a new background thread for database inserts
    let _thr_db_send = thread::spawn(move || {
        // Note: ownership of iopt is moved into this closure
        let bin = iopt.get("binary").unwrap();
        match bin.starts_with('/') {
            true => {
                info!("Using external Influx binary {}", bin);
                db_send_external(interval, &iopt)
            },
            _ => {
                info!("Using the internal InfluxDB client");
                db_send_internal(interval, &iopt)
            },
        }
    });
}

// Use the Rust native influxdb client library
fn db_send_internal(interval: i64, iopt: &InfOpt) {
    let meas = iopt.get("measurement").unwrap();
    let url = iopt.get("url").unwrap();
    let token = iopt.get("token").unwrap();
    let org = iopt.get("org").unwrap();
    let bucket = iopt.get("bucket").unwrap();
    let rt = Runtime::new().unwrap();

    loop {
        let waitsec = interval - (Utc::now().timestamp() % interval);
        // wait until next interval start
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_internal() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % interval);

        let mut pts = Vec::new();
        for sensorid in sensordata::sensor_list3() {
            pts.push(
                influxdb_client::Point::new(meas)
                    .tag("sensor", sensorid.as_str())
                    .field(
                        "value",
                        sensordata::get_avg(&sensorid, sensordata::get_avg_t_db()).unwrap(),
                    )
                    .timestamp(ts_i)
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
                info!("influxdb_client: {:?}", &pts);
                let res = c
                    .insert_points(&pts, influxdb_client::TimestampOptions::FromPoint)
                    .await;
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        error!("InfluxDB client error: {:?}", e);
                    }
                }
            });
        }
    }
}

// Run the external influx command to write data.
fn influx_run_cmd(iopt: &InfOpt, line_data: &str) {
    let binary = iopt.get("binary").unwrap();
    let host = iopt.get("url").unwrap();
    let token = iopt.get("token").unwrap();
    let org = iopt.get("org").unwrap();
    let bucket = iopt.get("bucket").unwrap();

    let iargs = [
        "write",
        "--precision", "s",
        "--host", host,
        "--token", token,
        "--org", org,
        "--bucket", bucket,
    ];
    info!("Running {} {}", binary, iargs.join(" "));
    info!("data:\n{}", line_data);
    let mut p = Command::new(binary)
        .args(&iargs)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let p_in = p.stdin.as_mut().unwrap();
    p_in.write_all(line_data.as_bytes()).unwrap();
    let out = p.wait_with_output().unwrap();
    if !out.status.success() || !out.stdout.is_empty() || !out.stderr.is_empty() {
        error!(
            "influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
            out.status.code().unwrap(),
            String::from_utf8(out.stderr).unwrap(),
            String::from_utf8(out.stdout).unwrap()
        );
    }
}

fn db_send_external(interval: i64, iopt: &InfOpt) {
    let meas = iopt.get("measurement").unwrap();
    let mut line_data = String::new();
    loop {
        let waitsec = interval - (Utc::now().timestamp() % interval);
        // wait until next interval start
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_ext() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % interval);

        line_data.clear();
        for sensorid in sensordata::sensor_list3() {
            line_data.push_str(
                format!(
                    "{},sensor={} value={:.2} {}\n",
                    meas,
                    &sensorid,
                    sensordata::get_avg(&sensorid, sensordata::get_avg_t_db()).unwrap(),
                    ts_i
                )
                .as_str(),
            );
        }
        // Only send if we have anything to send...
        if !line_data.is_empty() {
            influx_run_cmd(iopt, line_data.trim_end());
        }
    }
}
// EOF
