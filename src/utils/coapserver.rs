// utils/coapserver.rs

use log::*;
use std::lazy::*;
use std::sync::*;

use coap::Server;
use coap_lite::{RequestType as Method, CoapRequest, CoapResponse};
use tokio::runtime::Runtime;

use crate::utils::outsensor;
use crate::utils::sensordata;
use crate::utils::urlmap;
use std::net::SocketAddr;


const LISTEN_ADDR: &str = "0.0.0.0:5683";

fn resp_store_temp(payload: Option<&str>, code: &mut String, resp: &mut String) {
    match payload {
        None => {
            *code = "4.00".to_string();
            *resp = "NO DATA".to_string();
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
                },
                Ok(temp) => {
                    let sensorid = indata[0];
                    sensordata::add(sensorid, temp);
                    *code = "2.05".to_string();
                    *resp = "OK".to_string();
                },
            }
        },
    }
}

fn resp_list_sensors(_payload: Option<&str>, code: &mut String, resp: &mut String) {
    *code = "2.05".to_string();
    *resp = sensordata::sensor_list().join(" ");
}

fn resp_avg_out(_payload: Option<&str>, code: &mut String, resp: &mut String) {
    let skey = outsensor::get();
    let sdata = sensordata::get_avg15(&skey);

    match sdata {
        None => {
            *code = "5.03".to_string();
            *resp = "NO DATA".to_string();
        },
        Some(avg) => {
            let avg_out = format!("{:.2}", avg);
            *code = "2.05".to_string();
            *resp = avg_out;
        },
    }
}

fn resp_set_outsensor(payload: Option<&str>, code: &mut String, resp: &mut String) {
    match payload {
        None => {
            *code = "4.00".to_string();
            *resp = "NO DATA".to_string();
        },
        Some(data) => {
            outsensor::set(data);
            *code = "2.05".to_string();
            *resp = "OK".to_string();
        },
    }
}

fn resp_dump(_payload: Option<&str>, code: &mut String, resp: &mut String) {
    sensordata::dump();
    *code = "2.05".to_string();
    *resp = "OK".to_string();
}

static CNT: SyncLazy<Mutex<u64>> = SyncLazy::new(|| Mutex::new(0u64));

async fn handle_coap_req(request: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
    let i_save;
    {
        let mut i = CNT.lock().unwrap();
        *i += 1;
        i_save = *i;
    }
    let url_path = request.get_path();
    let mut resp_code = String::new();
    let mut resp= String::new();
    let ip_s;
    match request.source {
        None => {
            ip_s = "<none>".to_string();
        },
        Some(ip) => {
            ip_s = ip.to_string();
        },
    }
    info!("#{} {} {:?} /{}", i_save, ip_s, request.get_method(), url_path);

    match request.get_method() {
        &Method::Get => {
            urlmap::get(url_path.as_str())(None, &mut resp_code, &mut resp);
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
                    urlmap::get(url_path.as_str())(Some(&payload), &mut resp_code, &mut resp);
                },
            }
        },
        _ => {
            error!("--> Unsupported CoAP method {:?}", request.get_method());
            resp_code = "4.00".to_string();
            resp = "BAD REQUEST".to_string();
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
}

pub fn init() {
    info!("coapserver::init()");
    urlmap::init();
    info!("initializing url handlers");
    urlmap::add("store_temp", resp_store_temp);
    urlmap::add("list_sensors", resp_list_sensors);
    urlmap::add("avg_out", resp_avg_out);
    urlmap::add("set_outsensor", resp_set_outsensor);
    urlmap::add("dump", resp_dump);
    info!("Have {} URL responders.", urlmap::len());
    {
        // reset request counter
        let mut i = CNT.lock().unwrap();
        *i = 0;
    }
}

pub fn serve_coap() {
    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let mut server = Server::new(LISTEN_ADDR).unwrap();
        info!("Server up on {}", LISTEN_ADDR);
        server.run(handle_coap_req).await.unwrap();
    });
}
// EOF
