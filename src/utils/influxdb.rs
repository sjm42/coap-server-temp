// utils/influxdb.rs

use log::*;
use chrono::*;
use std::io::Write;
use std::process::*;
use std::{thread, time};
use async_std::*;
use influxdb_client::*;

use crate::utils::sensordata;


const INFLUX_BINARY: &str = "/usr/bin/influx";
const INFLUXDB_BUCKET: &str = "temperature";
const INFLUXDB_MEASUREMENT: &str = "temperature";

const INFLUXDB_URL: &str = "http://localhost:8086";
const INFLUXDB_TOKEN: &str = "W1o2562R92QdkcGmZOGiMROv_JIb773tS_wskzUed7bLJuOVVJ9y2rBKvaY3r7zmzIK7flzyW1F6SlRTqsJDYw==";
const INFLUXDB_ORG: &str = "siuro";


#[allow(dead_code)]
fn influx_send_ext(data: &Vec<String>) {
    // Run the external influx command to write data.
    // Here we assume that the user running this has the necessary InfluxDB client configs
    // available in home directory, including URL, Organization and Token.

    // This is clumsy, but has to be done this way because influxdb2 compatible client libraries
    // seem to need a different version of tokio library than coap server lib
    // and thus we would end up in dependency hell.

    // Luckily, this is only done once per minute, so it is not a performance issue.

    let line_data = data.iter().map(|s| &**s).collect::<Vec<_>>().join("\n");
    info!("IDB line data:\n{}", line_data);

    let mut p = Command::new(INFLUX_BINARY).arg("write")
        .arg("--precision").arg("s")
        .arg("--bucket").arg(INFLUXDB_BUCKET)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().unwrap();
    let p_in = p.stdin.as_mut().unwrap();
    p_in.write_all(line_data.as_bytes()).unwrap();
    let out = p.wait_with_output().unwrap();
    if !out.status.success() || out.stdout.len() > 0 || out.stderr.len() > 0 {
        error!("influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
               out.status.code().unwrap(),
               String::from_utf8(out.stderr).unwrap(),
               String::from_utf8(out.stdout).unwrap());
    }
}

#[allow(dead_code)]
fn db_send_ext() {
    let mut points = vec![];
    loop {
        let now = Utc::now();
        let waitsec = 60 - now.second();
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_ext() active");
        let ts = Utc::now().timestamp();
        let ts60 = ts - (ts % 60);

        points.clear();
        for sensorid in sensordata::sensor_list3() {
            points.push(format!("{},sensor={} value={:.2} {}", INFLUXDB_MEASUREMENT, sensorid, sensordata::get_avg5(&sensorid).unwrap(), ts60));
        }
        // Only send if we have anything to send...
        if points.len() > 0 {
            influx_send_ext(&points);
        }
    }
}

// Sadly, the Rust native influxdb client won't work with task::block_on() - the error message is:
// thread '<unnamed>' panicked at 'there is no reactor running, must be called from the context of a Tokio 1.x runtime',
// /home/sjm/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.6.0/src/runtime/blocking/pool.rs:85:33
#[allow(dead_code)]
fn db_send_native() {
    let mut pts = Vec::new();
    loop {
        let now = Utc::now();
        let waitsec = 60 - now.second();
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        trace!("influxdb::db_send_native() active");
        let ts = Utc::now().timestamp();
        let ts60 = ts - (ts % 60);

        pts.clear();
        for sensorid in sensordata::sensor_list3() {
            let p = Point::new(INFLUXDB_MEASUREMENT)
                .tag("sensor", sensorid.as_str())
                .field("value", sensordata::get_avg5(&sensorid).unwrap() as f64)
                .timestamp(ts60);
            pts.push(p);
        }
        if pts.len() > 0 {
            let c = Client::new(INFLUXDB_URL, INFLUXDB_TOKEN)
                .with_org(INFLUXDB_ORG)
                .with_bucket(INFLUXDB_BUCKET)
                .with_precision(Precision::S);
            let f = c.insert_points(&pts, TimestampOptions::FromPoint);
            let res = task::block_on(f);
            match res {
                Ok(_) => {},
                Err(e) => {
                    error!("InfluxDB client error: {:?}", e);
                },
            }
        }
    }
}

#[allow(dead_code)]
pub fn init() {
    trace!("influxdb::init() called");
    let _thr_db_send = thread::spawn(|| {
        // Use either of these:
        // db_send_native();
        db_send_ext();
    });

}
// EOF
