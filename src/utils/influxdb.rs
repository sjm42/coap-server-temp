// utils/influxdb.rs

use async_std::*;
use chrono::*;
use influxdb_client::*;
use log::*;
use std::collections::HashMap;
use std::io::Write;
use std::process::*;
use std::{thread, time};

use crate::utils::options;
use crate::utils::sensordata;

type InfOpt = HashMap<&'static str, String>;

pub fn init(interval: i64, opt: &options::CoapOpt) {
    trace!("influxdb::init()");
    let mut iopt: InfOpt = HashMap::new();
    iopt.insert("binary", opt.influxdb_binary.clone());
    iopt.insert("token", opt.influxdb_token.clone());
    iopt.insert("org", opt.influxdb_org.clone());
    iopt.insert("bucket", opt.influxdb_bucket.clone());
    iopt.insert("measurement", opt.influxdb_measurement.clone());
    iopt.insert("url", opt.influxdb_url.clone());
    let _thr_db_send = thread::spawn(move || {
        // Use either of these:
        match false {
            true => db_send_native(interval, &iopt),
            false => db_send_ext(interval, &iopt),
        }
    });
}

fn influx_send_ext(binary: &str, bucket: &str, line_data: &str) {
    // Run the external influx command to write data.
    // Here we assume that the user running this has the necessary InfluxDB client configs
    // available in home directory, including URL, Organization and Token.

    // This is clumsy, but has to be done this way because influxdb2 compatible client libraries
    // seem to need a different version of tokio library than coap server lib
    // and thus we would end up in dependency hell.

    // Luckily, this is only done once per minute, so it is not a performance issue.

    let iargs = ["write", "--precision", "s", "--bucket", bucket];
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

fn db_send_ext(interval: i64, iopt: &InfOpt) {
    let binary = iopt.get("binary").unwrap();
    let bucket = iopt.get("bucket").unwrap();
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
            influx_send_ext(binary, bucket, line_data.trim_end());
        }
    }
}

// Sadly, the Rust native influxdb client won't work with task::block_on() - the error message is:
// thread '<unnamed>' panicked at 'there is no reactor running, must be called from the context of a Tokio 1.x runtime',
// /home/sjm/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.6.0/src/runtime/blocking/pool.rs:85:33
#[allow(dead_code)]
fn db_send_native(interval: i64, iopt: &InfOpt) {
    let url = iopt.get("url").unwrap();
    let token = iopt.get("token").unwrap();
    let org = iopt.get("org").unwrap();
    let bucket = iopt.get("bucket").unwrap();
    let meas = iopt.get("measurement").unwrap();

    let mut pts = Vec::new();
    loop {
        let waitsec = interval - (Utc::now().timestamp() % interval);
        // wait until next interval start
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_native() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % interval);

        pts.clear();
        for sensorid in sensordata::sensor_list3() {
            let p = Point::new(meas)
                .tag("sensor", sensorid.as_str())
                .field(
                    "value",
                    sensordata::get_avg(&sensorid, sensordata::get_avg_t_db()).unwrap(),
                )
                .timestamp(ts_i);
            pts.push(p);
        }
        if !pts.is_empty() {
            let c = Client::new(url, token)
                .with_org(org)
                .with_bucket(bucket)
                .with_precision(Precision::S);
            let f = c.insert_points(&pts, TimestampOptions::FromPoint);
            let res = task::block_on(f);
            match res {
                Ok(_) => {}
                Err(e) => {
                    error!("InfluxDB client error: {:?}", e);
                }
            }
        }
    }
}
// EOF
