// main.rs
#![feature(async_closure)]
#![feature(once_cell)]

extern crate lazy_static;
extern crate coap;
extern crate chrono;
extern crate influxdb_client;

use std::{collections::HashMap, lazy::*, sync::*, thread, time};
use lazy_static::lazy_static;

use chrono::prelude::*;
use coap_lite::{RequestType as Method};
use coap::Server;
use tokio::runtime::Runtime;
use influxdb_client::*;

mod utils;
use utils::tbuf::*;
use utils::util::*;


const DB_URL: &str = "http://asdf.example.com:8086";
const DB_TOKEN: &str = "example-asdf-zxcv-example";
const DB_ORG: &str = "siuro";
const DB_BUCKET: &str = "temperature";
const DEFAULT_OUTSENSOR: &str = "28F41A2800008091";

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
        // mylog(&format!("db_send"));
        let now: DateTime<Utc> = Utc::now();
        let waitsec = 60 - now.second();
        thread::sleep(time::Duration::from_secs(waitsec as u64));

        points.clear();
        {
            let sd = SDATA.lock().unwrap();
            for (sensorid, tbuf) in sd.iter() {
                // Only send updates if we have at least 3 values in buffer!
                if tbuf.len() >= 3 {
                    points.push(Point::new("temperature")
                        .tag("sensor", sensorid.as_str())
                        .field("value", tbuf.avg5() as f64));
                }
            }
        }

        // Only send if we have anything to send...
        if points.len() > 0 {
            mylog(&format!("IDB: {:?}", points));
            let idbc = Client::new(DB_URL, DB_TOKEN)
                .with_org(DB_ORG)
                .with_bucket(DB_BUCKET)
                .with_precision(Precision::S);
            let _res = idbc.insert_points(&points, TimestampOptions::None);
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
