// main.rs
#![feature(async_closure)]
#![feature(once_cell)]

extern crate log;
extern crate simplelog;
extern crate lazy_static;
extern crate coap;
extern crate chrono;
extern crate influxdb_client;

use log::*;
use simplelog::*;
use std::collections::HashMap;
use std::lazy::*;
use std::process::{Command, Stdio};
use std::sync::*;
use std::thread;
use std::time;
use async_std::*;

use chrono::prelude::*;
use coap::Server;
use coap_lite::{RequestType as Method};
use lazy_static::lazy_static;
use tokio::runtime::Runtime;
use influxdb_client::*;

mod utils;
use utils::tbuf::*;
use std::io::Write;


const LISTEN_ADDR: &str = "0.0.0.0:5683";
const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";
const INFLUX_BINARY: &str = "/usr/bin/influx";
const INFLUXDB_BUCKET: &str = "temperature";
const INFLUXDB_MEASUREMENT: &str = "temperature";

const INFLUXDB_URL: &str = "http://localhost:8086";
const INFLUXDB_TOKEN: &str = "W1o2562R92QdkcGmZOGiMROv_JIb773tS_wskzUed7bLJuOVVJ9y2rBKvaY3r7zmzIK7flzyW1F6SlRTqsJDYw==";
const INFLUXDB_ORG: &str = "siuro";

type SensorData = HashMap<String, Tbuf>;
static SDATA: SyncLazy<Mutex<SensorData>> = SyncLazy::new(|| Mutex::new(SensorData::new()));
static OUTSENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));


fn resp_store_temp(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("store_temp payload={}", payload.unwrap_or(&"<none>".to_string()));
    match payload {
        None => {
            *code = "4.00".to_string();
            *resp = "NO DATA".to_string();
            return;
        },
        Some(data) => {
            let indata: Vec<&str> = data.split_whitespace().collect();
            if indata.len() != 2 {
                *code = "4.00".to_string();
                *resp = "ILLEGAL DATA".to_string();
                return;
            }
            match indata[1].parse::<f32>() {
                Err(_) => {
                    *code = "4.00".to_string();
                    *resp = "ILLEGAL DATA".to_string();
                    return;
                },
                Ok(temp) => {
                    let sensorid = indata[0];
                    let mut sd = SDATA.lock().unwrap();
                    if !sd.contains_key(sensorid) {
                        let new_tbuf = Tbuf::new();
                        sd.insert(sensorid.to_string(), new_tbuf);
                    }
                    let tbuf = sd.get_mut(sensorid).unwrap();
                    tbuf.add(Tdata::new(temp));
                    *code = "2.05".to_string();
                    *resp = "OK".to_string();
                    return;
                },
            }
        },
    }
}

fn resp_list_sensors(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("list_sensors payload={}", payload.unwrap_or(&"<none>".to_string()));
    let sd = SDATA.lock().unwrap();
    let sensor_list = sd.keys().map(|s| &**s).collect::<Vec<_>>().join(" ");
    *code = "2.05".to_string();
    *resp = sensor_list;
}

fn resp_avg_out(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("avg_out payload={}", payload.unwrap_or(&String::from("<none>")));
    let skey = OUTSENSOR.lock().unwrap();
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(&*skey) {
        *code = "5.03".to_string();
        *resp = "NO DATA".to_string();
        return;
    }
    let avg_out = format!("{:.2}", sd.get(&*skey).unwrap().avg15());
    *code = "2.05".to_string();
    *resp = avg_out;
}

fn resp_set_outsensor(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("set_outsensor payload={}", payload.unwrap_or(&"<none>".to_string()));
    match payload {
        None => {
            *code = "4.00".to_string();
            *resp = "NO DATA".to_string();
            return;
        },
        Some(data) => {
            {
                let mut s = OUTSENSOR.lock().unwrap();
                *s = data.to_string();
            }
            *code = "2.05".to_string();
            *resp = "OK".to_string();
            return;
        },
    }
}

fn resp_dump(payload: Option<&str>, code: &mut String, resp: &mut String) {
    trace!("dump payload={}", payload.unwrap_or(&"<none>".to_string()));
    let sd = SDATA.lock().unwrap();
    info!("dump: {} sensors.", sd.len());
    for (sensorid, tbuf) in sd.iter() {
        info!("dump: sensor {} tbuf={:?}", sensorid, tbuf);
    }
    *code = "2.05".to_string();
    *resp = "OK".to_string();
}

lazy_static! {
    static ref URLMAP: HashMap<&'static str, fn(Option<&str>, &mut String, &mut String)> = {
        trace!("URLMAP initializing");
        let mut m: HashMap<&'static str, fn(Option<&str>, &mut String, &mut String)> = HashMap::new();
        m.insert("store_temp", resp_store_temp);
        m.insert("list_sensors", resp_list_sensors);
        m.insert("avg_out", resp_avg_out);
        m.insert("set_outsensor", resp_set_outsensor);
        m.insert("dump", resp_dump);
        // INSERT MORE RESPONDER FUNCTION MAPPINGS HERE
        m
    };
}

// This is run in its own thread while program is running
fn tbuf_expire() {
    loop {
        // trace!("tbuf_expire active");
        {
            let mut sd = SDATA.lock().unwrap();
            for (_sensorid, tbuf) in sd.iter_mut() {
                let len1 = tbuf.len();
                if tbuf.expire() { tbuf.upd_avg(); }
                let n_exp = len1 - tbuf.len();
                if n_exp > 0 {
                    trace!("Expired: sensor {} n_exp={}", _sensorid, n_exp);
                }
            }
        }
        thread::sleep(time::Duration::from_secs(10));
    }
}

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

        // trace!("db_send_ext active");
        let ts = Utc::now().timestamp();
        let ts60 = ts - (ts % 60);

        points.clear();
        {
            let sd = SDATA.lock().unwrap();
            for (sensorid, tbuf) in sd.iter() {
                // Only send updates if we have some values in buffer!
                if tbuf.len() >= 3 {
                    points.push(format!("{},sensor={} value={:.2} {}", INFLUXDB_MEASUREMENT, sensorid, tbuf.avg5(), ts60));
                }
            }
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

        // trace!("db_send_native active");
        let ts = Utc::now().timestamp();
        let ts60 = ts - (ts % 60);

        pts.clear();
        {
            let sd = SDATA.lock().unwrap();
            for (sensorid, tbuf) in sd.iter() {
                // Only send updates if we have some values in buffer!
                if tbuf.len() >= 3 {
                    let p = Point::new(INFLUXDB_MEASUREMENT)
                        .tag("sensor", sensorid.as_str())
                        .field("value", tbuf.avg5() as f64)
                        .timestamp(ts60);
                    pts.push(p);
                }
            }
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


fn main() {
    SimpleLogger::init(LevelFilter::Info,
                       ConfigBuilder::new()
                             .set_time_format_str("%Y-%m-%d %H:%M:%S")
                             .build()).unwrap();
    // Here we are triggering the lazy initializations
    let n_url = URLMAP.len();
    let _n_sensors = SDATA.lock().unwrap().len();
    {
        let mut s = OUTSENSOR.lock().unwrap();
        *s = DEFAULT_OUTSENSOR.to_string();
    }
    info!("Have {} URL responders.", n_url);

    // Spawn some housekeeping threads
    let _thr_tbuf_expire = thread::spawn(|| {
        tbuf_expire();
    });
    let _thr_db_send = thread::spawn(|| {
        // Use either of these:
        // db_send_native();
        db_send_ext();
    });

    Runtime::new().unwrap().block_on(async move {
        let mut server = Server::new(LISTEN_ADDR).unwrap();
        info!("Server up on {}", LISTEN_ADDR);

        server.run(async move |request| {
            let url_path = request.get_path();
            let mut resp_code = "2.05".to_string();
            let mut resp = "".to_string();
            let ip_s: String;
            match request.source {
                None => {
                    ip_s = "<none>".to_string();
                },
                Some(ip) => {
                    ip_s = ip.to_string();
                },
            }
            info!("{} {:?} /{}", ip_s, request.get_method(), url_path);

            match URLMAP.get(url_path.as_str()) {
                None => {
                    resp_code = "4.04".to_string();
                    resp = "NOT FOUND".to_string();
                },
                Some(responder_f) => {
                    match request.get_method() {
                        &Method::Get => {
                            responder_f(None, &mut resp_code, &mut resp);
                        },
                        &Method::Post => {
                            let payload_o = String::from_utf8(request.message.payload);
                            match payload_o {
                                Err(e) => {
                                    error!("--> UTF-8 decode error: {:?}", e);
                                    resp_code = "4.00".to_string();
                                    resp = "BAD REQUEST".to_string();
                                },
                                Ok(payload) => {
                                    info!("<-- payload: {}", payload);
                                    responder_f(Some(&payload), &mut resp_code, &mut resp);
                                },
                            }
                        },
                        _ => {
                            error!("--> Unsupported CoAP method {:?}", request.get_method());
                            resp_code = "4.00".to_string();
                            resp = "BAD REQUEST".to_string();
                        },
                    }
                },
            }
            info!("--> {} {}", resp_code, resp);
            let resp_b = resp.as_bytes();
            return match request.response {
                Some(mut message) => {
                    message.message.header.set_code(&resp_code);
                    message.message.payload = resp_b.to_vec();
                    Some(message)
                },
                _ => None
            };
        }).await.unwrap();
    });
}
// EOF
