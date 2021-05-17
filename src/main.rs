// main.rs
#![feature(async_closure)]
#![feature(once_cell)]

extern crate lazy_static;
extern crate coap;
extern crate chrono;

use std::collections::HashMap;
use std::lazy::*;
use std::process::{Command, Stdio};
use std::sync::*;
use std::thread;
use std::time;

use chrono::prelude::*;
use coap::Server;
use coap_lite::{RequestType as Method};
use lazy_static::lazy_static;
use tokio::runtime::Runtime;

mod utils;
use utils::tbuf::*;
use utils::util::*;
use std::io::Write;


const DB_BUCKET: &str = "temperature";
const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";
const INFLUX: &str = "/usr/bin/influx";

type SensorData = HashMap<String, Tbuf>;
static SDATA: SyncLazy<Mutex<SensorData>> = SyncLazy::new(|| Mutex::new(SensorData::new()));
static OUTSENSOR: SyncLazy<Mutex<String>> = SyncLazy::new(|| Mutex::new(String::new()));


fn resp_store_temp(payload: Option<&str>) -> String {
    // mylog(&format!("store_temp payload={}", payload.unwrap_or(&"<none>".to_string())));
    match payload {
        None => {
            return "NO DATA".to_string();
        },
        Some(data) => {
            let indata: Vec<&str> = data.split_whitespace().collect();
            if indata.len() != 2 {
                return "ILLEGAL DATA".to_string();
            }
            match indata[1].parse::<f32>() {
                Err(_) => {
                    return "ILLEGAL DATA".to_string();
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
                    return "OK".to_string();
                },
            }
        },
    }
}

fn resp_list_sensors(_payload: Option<&str>) -> String {
    // mylog(&format!("list_sensors payload={}", _payload.unwrap_or(&"<none>".to_string())));
    let sd = SDATA.lock().unwrap();
    let sensor_list = sd.keys().map(|s| &**s).collect::<Vec<_>>().join(" ");
    mylog(&sensor_list);
    sensor_list
}

fn resp_avg_out(_payload: Option<&str>) -> String {
    // mylog(&format!("avg_out payload={}", _payload.unwrap_or(&String::from("<none>"))));
    let skey = OUTSENSOR.lock().unwrap();
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(&*skey) {
        return "NO DATA".to_string()
    }
    let avg_out = format!("{:.2}", sd.get(&*skey).unwrap().avg15());
    mylog(&avg_out);
    avg_out
}

fn resp_set_outsensor(payload: Option<&str>) -> String {
    // mylog(&format!("set_outsensor payload={}", payload.unwrap_or(&"<none>".to_string())));
    match payload {
        None => {
            return "NO DATA".to_string();
        },
        Some(data) => {
            let mut s = OUTSENSOR.lock().unwrap();
            *s = data.to_string();
            return "OK".to_string();
        },
    }
}

fn resp_dump(_payload: Option<&str>) -> String {
    let sd = SDATA.lock().unwrap();
    mylog(&format!("Have {} sensors.", sd.len()));
    for (sensorid, tbuf) in sd.iter() {
        mylog(&format!("sensor {} tbuf={:?}", sensorid, tbuf));
    }
    "OK".to_string()
}

lazy_static! {
    static ref URLMAP: HashMap<&'static str, fn(Option<&str>)->String> = {
        // mylog(format!("URLMAP initializing"));
        let mut m: HashMap<&'static str, fn(Option<&str>)->String> = HashMap::new();
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
        // mylog(&format!("buf_expire"));
        {
            let mut sd = SDATA.lock().unwrap();
            for (_sensorid, tbuf) in sd.iter_mut() {
                let len1 = tbuf.len();
                if tbuf.expire() { tbuf.upd_avg(); }
                let n_exp = len1 - tbuf.len();
                if n_exp > 0 {
                    // mylog(&format!("Expired: sensor {} n_exp={}", _sensorid, n_exp));
                }
            }
        }
        thread::sleep(time::Duration::from_secs(10));
    }
}

// This is run in its own thread while program is running
fn db_send() {
    let mut points = vec![];
    loop {
        let now = Utc::now();
        let waitsec = 60 - now.second();
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        let ts = Utc::now().timestamp();
        let ts60 = ts - (ts % 60);

        points.clear();
        {
            let sd = SDATA.lock().unwrap();
            for (sensorid, tbuf) in sd.iter() {
                // Only send updates if we have some values in buffer!
                if tbuf.len() >= 3 {
                    points.push(format!("temperature,sensor={} value={:.2} {}", sensorid, tbuf.avg5(), ts60));
                }
            }
        }

        // Only send if we have anything to send...
        if points.len() > 0 {
            let line_data = points.iter().map(|s| &**s).collect::<Vec<_>>().join("\n");
            // mylog(&format!("IDB: {:?}", points));
            mylog(&format!("IDB line data:\n{}", line_data));

            // Run the external influx command to write data.
            // This is clumsy, but has to be done this way because influxdb2 compatible client libraries
            // seem to need a different version of tokio library than coap server lib
            // and thus we would end up in dependency hell.
            let mut p = Command::new(INFLUX).arg("write")
                .arg("--precision").arg("s")
                .arg("--bucket").arg(DB_BUCKET)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn().unwrap();
            let p_in = p.stdin.as_mut().unwrap();
            p_in.write_all(line_data.as_bytes()).unwrap();
            drop(p_in); // close stdin already -- probably not needed anymore these days
            let out = p.wait_with_output().unwrap();
            if !out.status.success() || out.stdout.len() > 0 || out.stderr.len() > 0 {
                mylog(&format!("influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
                    out.status.code().unwrap(),
                    String::from_utf8(out.stderr).unwrap(),
                    String::from_utf8(out.stdout).unwrap()));
            }
        }
    }
}


fn main() {
    let addr = "0.0.0.0:5683";

    // Here we are triggering the lazy initializations
    let n_url = URLMAP.len();
    let _n_sensors = SDATA.lock().unwrap().len();
    {
        let mut s = OUTSENSOR.lock().unwrap();
        *s = DEFAULT_OUTSENSOR.to_string();
    }
    mylog(&format!("Have {} URL responders.", n_url));

    // Spawn some housekeeping threads
    let _thr_tbuf_expire = thread::spawn(|| {
        tbuf_expire();
    });
    let _thr_db_send = thread::spawn(|| {
        db_send();
    });

    Runtime::new().unwrap().block_on(async move {
        let mut server = Server::new(addr).unwrap();
        mylog(&format!("Server up on {}", addr));

        server.run(async move |request| {
            let url_path = request.get_path();
            let mut resp = String::from("");
            let mut resp_code = "2.05";

            match URLMAP.get(url_path.as_str()) {
                None => {
                    resp = "NOT FOUND".to_string();
                    resp_code = "4.04";
                },
                Some(responder_f) => {
                    match request.get_method() {
                        &Method::Get => {
                            mylog(&format!("GET /{}", url_path));
                            resp = responder_f(None);
                            mylog(&format!("--> {}", resp))
                        },
                        &Method::Post => {
                            let payload_o = String::from_utf8(request.message.payload);
                            match payload_o {
                                Err(e) => {
                                    mylog(&format!("UTF-8 decode error: {}", e));
                                    resp = "BAD REQUEST".to_string();
                                    resp_code = "4.00";
                                },
                                Ok(payload) => {
                                    mylog(&format!("POST /{} data: {}", url_path, payload));
                                    resp = responder_f(Some(&payload));
                                    mylog(&format!("--> {}", resp))
                                },
                            }
                        },
                        _ => mylog(&"Unsupported CoAP method!".to_string()),
                    }
                },
            }

            let resp_b = resp.as_bytes();
            return match request.response {
                Some(mut message) => {
                    message.message.header.set_code(resp_code);
                    message.message.payload = resp_b.to_vec();
                    Some(message)
                },
                _ => None
            };
        }).await.unwrap();
    });
}
// EOF
