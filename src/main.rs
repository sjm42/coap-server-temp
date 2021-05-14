// main.rs
#![feature(async_closure)]
#![feature(once_cell)]

extern crate lazy_static;
extern crate coap;

use std::{lazy::SyncLazy, sync::Mutex};
use std::collections::HashMap;
use lazy_static::lazy_static;
use std::time::*;

use coap_lite::{RequestType as Method};
use coap::Server;
use tokio::runtime::Runtime;

mod utils;
use utils::tbuf::*;
use utils::util::*;


// This is scary.
static TBUF: SyncLazy<Mutex<Vec<Tbuf>>> = SyncLazy::new(|| Mutex::new(vec![]));

fn resp_list_sensors(_payload: Option<&String>) -> String {
    mylog(&format!("Sensor list! payload={}", _payload.unwrap_or(&String::from("<none>"))));
    // NOT IMPLEMENTED YET
    String::from("<sensorlist>")
}

fn resp_store_temp(payload: Option<&String>) -> String {
    mylog(&format!("Store temp! payload={}", payload.unwrap_or(&String::from("<none>"))));
    match payload {
        None => {
            return String::from("NO DATA");
        },
        Some(data) => {
            let indata: Vec<&str> = data.split_whitespace().collect();
            if indata.len() < 2 {
                return String::from("ILLEGAL DATA");
            }
            match indata[1].parse::<f32>() {
                Err(_) => {
                    return String::from("ILLEGAL DATA");
                },
                Ok(temp) => {
                    // indata[0] contains the sensor id -- not used just yet
                    let mut dv = TBUF.lock().unwrap();
                    mylog(&format!("TBUF len={}", dv.len()));
                    for tbuf in &mut *dv {
                        tbuf.add(Tdata { ts: SystemTime::now(), data: temp});
                    }
                    String::from("OK")
                },
            }
        },
    }
}

fn resp_get_avg(_payload: Option<&String>) -> String {
    mylog(&format!("get avg! payload={}", _payload.unwrap_or(&String::from("<none>"))));
    let avg: f64;
    {
        let dv = TBUF.lock().unwrap();
        let mut i = 0;
        avg = dv[0].avg;
        for tbuf in &*dv {
            mylog(&format!("avg {} is {:.2}", i, tbuf.avg));
            i += 1;
        }
    }
    format!("{:.2}", avg)
}

lazy_static! {
    static ref URLMAP: HashMap<String, fn(Option<&String>)->String> = {
        // mylog(format!("URLMAP initializing"));
        let mut m: HashMap<String, fn(Option<&String>)->String> = HashMap::new();
        m.insert(String::from("list_sensors"), resp_list_sensors);
        m.insert(String::from("store_temp"), resp_store_temp);
        m.insert(String::from("get_avg"), resp_get_avg);
        // INSERT MORE RESPONDER FUNCTION MAPPINGS HERE
        m
    };
}


fn main() {
    let addr = "127.0.0.1:5683";

    // Here we are triggering the lazy initializations
    let _ = URLMAP.len();
    {
        let mut dv = TBUF.lock().unwrap();
        dv.push(Tbuf::new(300));
        dv.push(Tbuf::new(900));
    }

    Runtime::new().unwrap().block_on(async move {
        let mut server = Server::new(addr).unwrap();
        mylog(&format!("Server up on {}", addr));

        server.run(async move |request| {
            let url_path = request.get_path();
            let mut resp = String::from("");
            let mut resp_code = "2.05";

            match URLMAP.get(&url_path) {
                None => {
                    resp = String::from("NOT FOUND");
                    resp_code = "4.04";
                },
                Some(responder_f) => {
                    match request.get_method() {
                        &Method::Get => {
                            mylog(&format!("GET /{}", url_path));
                            resp = responder_f(None);
                        },
                        &Method::Post => {
                            let payload_o = String::from_utf8(request.message.payload);
                            match payload_o {
                                Err(e) => {
                                    mylog(&format!("UTF-8 decode error: {}", e));
                                    resp = String::from("BAD REQUEST");
                                    resp_code = "4.00";
                                },
                                Ok(payload) => {
                                    mylog(&format!("POST /{} data: {}", url_path, payload));
                                    resp = responder_f(Some(&payload));
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
