// utils/coapserver.rs

use log::*;
use std::lazy::*;
use std::sync::*;

use coap_lite::{CoapRequest, CoapResponse, RequestType as Method, ResponseType};
use std::net::SocketAddr;
use tokio::runtime::Runtime;

use crate::utils::options;
use crate::utils::sensordata;
use crate::utils::urlmap::*;

// our global persistent state, with locking
static URLMAP: SyncLazy<Mutex<UrlMap>> = SyncLazy::new(|| Mutex::new(UrlMap::new_cap(8)));
static CNT: SyncLazy<Mutex<u64>> = SyncLazy::new(|| Mutex::new(0u64));

pub fn init(_opt: &options::GlobalServerOptions) {
    trace!("coapserver::init()");
    {
        info!("Creating url handlers");
        let mut urlmap = URLMAP.lock().unwrap();
        urlmap.clear();
        urlmap.add("store_temp", resp_store_temp);
        urlmap.add("list_sensors", resp_list_sensors);
        urlmap.add("avg_out", resp_avg_out);
        urlmap.add("set_outsensor", resp_set_outsensor);
        urlmap.add("dump", resp_dump);
        info!("Have {} URL responders.", urlmap.len());
    }
    {
        // reset the request counter
        let mut i = CNT.lock().unwrap();
        *i = 0;
    }
}

fn resp_store_temp(payload: Option<&str>) -> (ResponseType, String) {
    match payload {
        None => (ResponseType::BadRequest, "NO DATA".to_string()),
        Some(data) => {
            let indata: Vec<&str> = data.split_whitespace().collect();
            if indata.len() != 2 {
                return (ResponseType::BadRequest, "INVALID DATA".to_string());
            }
            match indata[1].parse::<f32>() {
                Err(_) => (ResponseType::BadRequest, "INVALID DATA".to_string()),
                Ok(temp) => {
                    sensordata::add(indata[0], temp);
                    (ResponseType::Content, "OK".to_string())
                }
            }
        }
    }
}

fn resp_list_sensors(_payload: Option<&str>) -> (ResponseType, String) {
    (ResponseType::Content, sensordata::sensor_list().join(" "))
}

fn resp_avg_out(_payload: Option<&str>) -> (ResponseType, String) {
    let t_out = sensordata::get_avg_out();
    match t_out {
        None => (ResponseType::ServiceUnavailable, "NO DATA".to_string()),
        Some(avg) => (ResponseType::Content, format!("{:.2}", avg)),
    }
}

fn resp_set_outsensor(payload: Option<&str>) -> (ResponseType, String) {
    match payload {
        None => (ResponseType::BadRequest, "NO DATA".to_string()),
        Some(data) => {
            sensordata::set_outsensor(data);
            (ResponseType::Content, "OK".to_string())
        }
    }
}

fn resp_dump(_payload: Option<&str>) -> (ResponseType, String) {
    sensordata::dump();
    (ResponseType::Content, "OK".to_string())
}

fn get_handler(url_path: &str) -> UrlHandler {
    let handler;
    {
        let urlmap = URLMAP.lock().unwrap();
        handler = urlmap.get(url_path);
    }
    handler
}

async fn handle_coap_req(request: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
    let i_save;
    {
        // increment the request counter and save the value after releasing the lock
        let mut i = CNT.lock().unwrap();
        *i += 1;
        i_save = *i;
    }
    let req_path = request.get_path();
    let url_path = req_path.as_str();
    let ip_tmp;
    let ip_str;
    let ret;
    let resp_code: ResponseType;
    let resp_data: &str;

    match request.source {
        None => {
            ip_str = "<none>";
        }
        Some(ip) => {
            ip_tmp = ip.to_string();
            ip_str = ip_tmp.as_str();
        }
    }
    info!(
        "#{} {} {:?} /{}",
        i_save,
        ip_str,
        request.get_method(),
        url_path
    );

    match *request.get_method() {
        Method::Get => {
            ret = get_handler(url_path)(None);
            resp_code = ret.0;
            resp_data = ret.1.as_str();
        }
        Method::Post => match String::from_utf8(request.message.payload) {
            Err(e) => {
                error!("--> UTF-8 decode error: {:?}", e);
                resp_code = ResponseType::BadRequest;
                resp_data = "INVALID UTF8";
            }
            Ok(payload) => {
                info!("<-- payload: {}", payload);
                ret = get_handler(url_path)(Some(&payload));
                resp_code = ret.0;
                resp_data = ret.1.as_str();
            }
        },
        _ => {
            error!("--> Unsupported CoAP method {:?}", request.get_method());
            resp_code = ResponseType::BadRequest;
            resp_data = "INVALID METHOD";
        }
    }
    info!("--> {:?} {}", resp_code, resp_data);

    match request.response {
        Some(mut message) => {
            message.set_status(resp_code);
            message.message.payload = resp_data.as_bytes().to_vec();
            trace!("--> {:?}", message);
            Some(message)
        }
        _ => None,
    }
}

pub fn serve_coap(opt: &options::GlobalServerOptions) {
    let listen = &opt.listen;
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let mut server = coap::Server::new(listen).unwrap();
        info!("Listening on {}", listen);
        info!("Server running...");
        server.run(handle_coap_req).await.unwrap();
    });
}
// EOF
