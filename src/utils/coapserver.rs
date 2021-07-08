// utils/coapserver.rs

use log::*;
use std::lazy::*;
use std::sync::*;

use coap_lite::{CoapRequest, CoapResponse, RequestType as Method, ResponseType};
use std::net::SocketAddr;
use tokio::runtime::Runtime;

use crate::utils::options;
use crate::utils::sensordata;
use crate::utils::url::*;

// our global persistent state, with locking
static URLMAP: SyncLazy<Mutex<UrlMap>> = SyncLazy::new(|| {
    Mutex::new(
        UrlMap::new()
            .with_map("store_temp", resp_store_temp)
            .with_map("list_sensors", resp_list_sensors)
            .with_map("avg_out", resp_avg_out)
            .with_map("set_outsensor", resp_set_outsensor)
            .with_map("dump", resp_dump),
    )
});

static CNT: SyncLazy<Mutex<u64>> = SyncLazy::new(|| Mutex::new(0u64));

pub fn init(_opt: &options::GlobalServerOptions) {
    trace!("coapserver::init()");
    {
        info!("Creating url handlers");
        let u = URLMAP.lock().unwrap();
        info!("Have {} URL responders.", u.len());
    }
    {
        let _i = CNT.lock().unwrap();
    }
}

fn resp_store_temp(payload: Option<&str>) -> UrlResponse {
    match payload {
        None => UrlResponse::new(ResponseType::BadRequest, "NO DATA"),
        Some(data) => {
            let indata: Vec<&str> = data.split_whitespace().collect();
            if indata.len() != 2 {
                return UrlResponse::new(ResponseType::BadRequest, "INVALID DATA");
            }
            match indata[1].parse::<f32>() {
                Err(_) => UrlResponse::new(ResponseType::BadRequest, "INVALID DATA"),
                Ok(temp) => {
                    sensordata::add(indata[0], temp);
                    UrlResponse::new(ResponseType::Content, "OK")
                }
            }
        }
    }
}

fn resp_list_sensors(_payload: Option<&str>) -> UrlResponse {
    UrlResponse::new(ResponseType::Content, sensordata::sensors_list().join(" "))
}

fn resp_avg_out(_payload: Option<&str>) -> UrlResponse {
    match sensordata::get_avg_out() {
        None => UrlResponse::new(ResponseType::ServiceUnavailable, "NO DATA"),
        Some(avg) => UrlResponse::new(ResponseType::Content, format!("{:.2}", avg)),
    }
}

fn resp_set_outsensor(payload: Option<&str>) -> UrlResponse {
    match payload {
        None => UrlResponse::new(ResponseType::BadRequest, "NO DATA"),
        Some(data) => {
            sensordata::set_outsensor(data);
            UrlResponse::new(ResponseType::Content, "OK")
        }
    }
}

fn resp_dump(_payload: Option<&str>) -> UrlResponse {
    sensordata::dump();
    UrlResponse::new(ResponseType::Content, "OK")
}

fn get_handler(url_path: &str) -> UrlHandler {
    URLMAP.lock().unwrap().get(url_path)
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

    let method = *request.get_method();
    match method {
        Method::Get => {
            // Call the URL handler without payload
            ret = get_handler(url_path)(None);
            resp_code = ret.code();
            resp_data = ret.data();
        }
        Method::Post => match String::from_utf8(request.message.payload) {
            Err(e) => {
                error!("--> UTF-8 decode error: {:?}", e);
                resp_code = ResponseType::BadRequest;
                resp_data = "INVALID UTF8";
            }
            Ok(payload) => {
                info!("<-- payload: {}", payload);
                // Call the URL handler with payload
                ret = get_handler(url_path)(Some(&payload));
                resp_code = ret.code();
                resp_data = ret.data();
            }
        },
        _ => {
            error!("--> Unsupported CoAP method {:?}", method);
            resp_code = ResponseType::BadRequest;
            resp_data = "INVALID METHOD";
        }
    }
    info!("--> {:?} {}", resp_code, resp_data);

    match request.response {
        Some(mut message) => {
            message.set_status(resp_code);
            message.message.payload = resp_data.into();
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
