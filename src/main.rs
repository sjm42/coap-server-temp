// main.rs

#![feature(async_closure)]

extern crate chrono;
extern crate lazy_static;
extern crate coap;

use chrono::prelude::*;
use std::collections::HashMap;
use lazy_static::lazy_static;

use coap_lite::{RequestType as Method};
use coap::Server;
use tokio::runtime::Runtime;


fn mylog(logentry: &String) {
    let timestamp: DateTime<Local> = Local::now();
    println!("{}: {}", timestamp.format("%Y-%m-%d %H:%M:%S"), logentry);
}

fn resp_list_sensors(_payload: Option<&String>) -> String {
    mylog(&format!("Sensor list! payload={}", _payload.unwrap_or(&String::from("<none>"))));
    String::from("<sensorlist>")
}

lazy_static! {
    static ref URLMAP: HashMap<String, fn(Option<&String>)->String> = {
        // mylog(format!("URLMAP initializing"));
        let mut m: HashMap<String, fn(Option<&String>)->String> = HashMap::new();
        m.insert(String::from("list_sensors"), resp_list_sensors);
        // INSERT MORE RESPONDER FUNCTION MAPPINGS HERE
        m
    };
}

fn main() {
    let addr = "127.0.0.1:5683";

    // Here we are triggering the lazy initialization of URLMAP.
    let _ = URLMAP.get("*");

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
