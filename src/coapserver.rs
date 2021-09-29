// coapserver.rs

use super::sensordata;
use super::startup;
use super::url::*;

use coap_lite::{CoapRequest, CoapResponse, RequestType as Method, ResponseType};
use log::*;
use once_cell::sync::Lazy;
use parking_lot::*;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;

// our global persistent state, with locking
static URLMAP: Lazy<RwLock<UrlMap>> = Lazy::new(|| {
    RwLock::new(
        UrlMap::new()
            .with_path("store_temp", resp_store_temp)
            .with_path("list_sensors", resp_list_sensors)
            .with_path("avg_out", resp_avg_out)
            .with_path("set_outsensor", resp_set_outsensor)
            .with_path("dump", resp_dump),
    )
});

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

static CNT: AtomicU64 = AtomicU64::new(0);

async fn handle_coap_req(request: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
    let i = CNT.fetch_add(1, Ordering::Relaxed);
    let req_path = &request.get_path();
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
            ip_str = &ip_tmp;
        }
    }
    let method = *request.get_method();
    info!("#{} {} {:?} /{}", i, ip_str, method, req_path);

    match method {
        Method::Get => {
            // Call the URL handler without payload
            ret = URLMAP.read().get_handler(req_path)(None);
            resp_code = ret.code();
            resp_data = ret.data();
        }
        Method::Post => {
            // Let's do relaxed UTF-8 conversion.
            let payload = &String::from_utf8_lossy(&request.message.payload);
            info!("<-- payload: {}", payload);
            // Call the URL handler with payload
            ret = URLMAP.read().get_handler(req_path)(Some(payload));
            resp_code = ret.code();
            resp_data = ret.data();
        }
        _ => {
            info!("--> Unsupported CoAP method {:?}", method);
            resp_code = ResponseType::BadRequest;
            resp_data = "INVALID METHOD";
        }
    }
    info!("--> {:?} {}", resp_code, resp_data);

    match request.response {
        Some(mut message) => {
            message.set_status(resp_code);
            message.message.payload = resp_data.into();
            debug!("--> {:?}", message);
            Some(message)
        }
        _ => None,
    }
}

pub fn run(opt: &startup::OptsCommon) -> anyhow::Result<()> {
    trace!("coapserver::run()");
    {
        info!("Creating url handlers");
        let u = URLMAP.read();
        info!("Have {} URL responders.", u.len());
        debug!("URL map:\n{:?}", u);
    }
    let listen = &opt.listen;
    let rt = Runtime::new()?;
    rt.block_on(async move {
        info!("Listening on {}", listen);
        info!("Server running...");
        let mut server = coap::Server::new(listen).unwrap();
        server.run(handle_coap_req).await.unwrap();
    });
    Ok(())
}
// EOF
