// main.rs
#![feature(async_closure)]
#![feature(once_cell)]

extern crate lazy_static;
extern crate coap;

use std::{lazy::SyncLazy, sync::Mutex};
use std::collections::HashMap;
use lazy_static::lazy_static;

use coap_lite::{RequestType as Method};
use coap::Server;
use tokio::runtime::Runtime;

mod utils;
use utils::tbuf::*;
use utils::util::*;

const OUT_SENSOR: &str = "28F41A2800008091";


// This is scary.
type SensorData = HashMap<String, Tbuf>;
static SDATA: SyncLazy<Mutex<SensorData>> = SyncLazy::new(|| Mutex::new(SensorData::new()));


fn resp_store_temp(payload: Option<&str>) -> String {
    // mylog(&format!("Store temp! payload={}", payload.unwrap_or(&"<none>".to_string())));
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
    // mylog(&format!("Sensor list! payload={}", _payload.unwrap_or(&"<none>".to_string())));
    let sd = SDATA.lock().unwrap();
    let sensors = sd.keys().map(|s| &**s).collect::<Vec<_>>().join(" ");
    mylog(&sensors);
    sensors
}

fn resp_avg_out(_payload: Option<&str>) -> String {
    // mylog(&format!("get avg! payload={}", _payload.unwrap_or(&String::from("<none>"))));
    let sd = SDATA.lock().unwrap();
    if !sd.contains_key(OUT_SENSOR) {
        return "NO DATA".to_string()
    }
    let a = format!("{:.2}", sd.get(OUT_SENSOR).unwrap().avg15());
    mylog(&a);
    a
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
        m.insert("dump", resp_dump);
        // INSERT MORE RESPONDER FUNCTION MAPPINGS HERE
        m
    };
}


fn main() {
    let addr = "127.0.0.1:5683";

    // Here we are triggering the lazy initializations
    let n_url = URLMAP.len();
    let _n_sensors = SDATA.lock().unwrap().len();

    mylog(&format!("Have {} URL responders.", n_url));

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
