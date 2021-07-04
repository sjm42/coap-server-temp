// utils/coapserver.rs

use log::*;
use std::lazy::*;
use std::sync::*;

use coap_lite::{CoapRequest, CoapResponse, RequestType as Method};
use std::net::SocketAddr;
use tokio::runtime::Runtime;

use crate::utils::sensordata;
use crate::utils::urlmap;

// our global persistent state, with locking
// we just have a simple request counter here
static CNT: SyncLazy<Mutex<u64>> = SyncLazy::new(|| Mutex::new(0u64));

fn resp_store_temp(payload: Option<&str>) -> (String, String) {
    match payload {
        None => ("4.00".to_string(), "NO DATA".to_string()),
        Some(data) => {
            let indata: Vec<&str> = data.split_whitespace().collect();
            if indata.len() != 2 {
                return ("4.00".to_string(), "ILLEGAL DATA".to_string());
            }
            match indata[1].parse::<f32>() {
                Err(_) => ("4.00".to_string(), "ILLEGAL DATA".to_string()),
                Ok(temp) => {
                    sensordata::add(indata[0], temp);
                    ("2.05".to_string(), "OK".to_string())
                }
            }
        }
    }
}

fn resp_list_sensors(_payload: Option<&str>) -> (String, String) {
    ("2.05".to_string(), sensordata::sensor_list().join(" "))
}

fn resp_avg_out(_payload: Option<&str>) -> (String, String) {
    let t_out = sensordata::get_avg_out();
    match t_out {
        None => ("5.03".to_string(), "NO DATA".to_string()),
        Some(avg) => ("2.05".to_string(), format!("{:.2}", avg)),
    }
}

fn resp_set_outsensor(payload: Option<&str>) -> (String, String) {
    match payload {
        None => ("4.00".to_string(), "NO DATA".to_string()),
        Some(data) => {
            sensordata::set_outsensor(data);
            ("2.05".to_string(), "OK".to_string())
        }
    }
}

fn resp_dump(_payload: Option<&str>) -> (String, String) {
    sensordata::dump();
    ("2.05".to_string(), "OK".to_string())
}

async fn handle_coap_req(request: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
    let i_save;
    {
        // increment the request counter and save the value after releasing the lock
        let mut i = CNT.lock().unwrap();
        *i += 1;
        i_save = *i;
    }
    let url_path = request.get_path();
    let ret;
    let resp_code: &str;
    let resp: &str;
    let ip_s;
    match request.source {
        None => {
            ip_s = "<none>".to_string();
        }
        Some(ip) => {
            ip_s = ip.to_string();
        }
    }
    info!(
        "#{} {} {:?} /{}",
        i_save,
        ip_s,
        request.get_method(),
        url_path
    );

    match *request.get_method() {
        Method::Get => {
            ret = urlmap::get(url_path.as_str())(None);
            resp_code = ret.0.as_str();
            resp = ret.1.as_str();
        }
        Method::Post => {
            let payload_o = String::from_utf8(request.message.payload);
            match payload_o {
                Err(e) => {
                    error!("--> UTF-8 decode error: {:?}", e);
                    resp_code = "4.00";
                    resp = "BAD REQUEST";
                }
                Ok(payload) => {
                    info!("<-- payload: {}", payload);
                    ret = urlmap::get(url_path.as_str())(Some(&payload));
                    resp_code = ret.0.as_str();
                    resp = ret.1.as_str();
                }
            }
        }
        _ => {
            error!("--> Unsupported CoAP method {:?}", request.get_method());
            resp_code = "4.00";
            resp = "BAD REQUEST";
        }
    }
    info!("--> {} {}", resp_code, resp);
    match request.response {
        Some(mut message) => {
            message.message.header.set_code(resp_code);
            let resp_b = resp.as_bytes();
            message.message.payload = resp_b.to_vec();
            Some(message)
        }
        _ => None,
    }
}

pub fn init() {
    trace!("coapserver::init()");
    urlmap::init();
    info!("Creating url handlers");
    urlmap::add("store_temp", resp_store_temp);
    urlmap::add("list_sensors", resp_list_sensors);
    urlmap::add("avg_out", resp_avg_out);
    urlmap::add("set_outsensor", resp_set_outsensor);
    urlmap::add("dump", resp_dump);
    info!("Have {} URL responders.", urlmap::len());
    {
        // reset the request counter
        let mut i = CNT.lock().unwrap();
        *i = 0;
    }
}

pub fn serve_coap(listen: &str) {
    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let mut server = coap::Server::new(listen).unwrap();
        info!("Listening on {}", listen);
        info!("Server running...");
        server.run(handle_coap_req).await.unwrap();
    });
}
// EOF
