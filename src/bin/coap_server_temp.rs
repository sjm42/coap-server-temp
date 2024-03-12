// bin/coap_server_temp.rs

use coap_server_temp::*;
use sensordata::{run_expire, MyData};

use clap::Parser;
use coap_lite::{CoapResponse, RequestType, ResponseType};
use coap_server::app::{self, CoapError, Request, Response};
use coap_server::CoapServer;
use coap_server_tokio::transport::udp::UdpTransport;
use influxdb::InfluxSender;
use log::*;
use once_cell::sync::OnceCell;
use std::sync::{atomic, Arc};
use std::{cmp::Ordering, net::SocketAddr};

pub struct MyState {
    mydata: Arc<MyData>,
    counter: atomic::AtomicU64,
}

// This is for CoAP server because coap-server crate does not carry any state
static MYSTATE: OnceCell<MyState> = OnceCell::new();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::parse();
    opts.finish()?;
    debug!("Global config: {opts:?}");
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let mydata = Arc::new(MyData::new(&opts));

    MYSTATE
        .set(MyState {
            mydata: mydata.clone(),
            counter: atomic::AtomicU64::new(0),
        })
        .ok();

    tokio::spawn(run_expire(mydata.clone(), opts.expire_interval));
    tokio::spawn(InfluxSender::new(&opts, mydata.clone()).run_db_send());

    let addr = opts.listen.to_string();

    let server = CoapServer::bind(UdpTransport::new(&addr)).await?;
    info!("Listening on {addr}");

    info!("Server running...");
    Ok(server
        .serve(
            app::new()
                .resource(app::resource("/avg_out").get(resp_get_avg_out))
                .resource(app::resource("/dump").get(resp_get_dump))
                .resource(app::resource("/sensor").get(resp_get_sensor))
                .resource(app::resource("/list_sensors").get(resp_get_list_sensors))
                .resource(app::resource("/set_outsensor").post(resp_post_set_outsensor))
                .resource(app::resource("/store_temp").post(resp_post_store_temp))
                .resource(app::resource("/store").post(resp_post_store_temp))
                .resource(app::resource("/").default_handler(resp_default)),
        )
        .await?)
}

fn mydata() -> Arc<MyData> {
    MYSTATE.get().unwrap().mydata.clone()
}

fn req_id() -> u64 {
    MYSTATE
        .get()
        .unwrap()
        .counter
        .fetch_add(1, atomic::Ordering::Relaxed)
}

fn log_request(request: &Request<SocketAddr>) {
    let id = req_id();
    let ip_str = match request.original.source {
        None => "<none>".into(),
        Some(ip) => ip.to_string(),
    };
    let method = *request.original.get_method();
    let path = request.original.get_path();
    info!("#{id} {ip_str} {method:?} /{path}");

    if let RequestType::Post = method {
        let data = String::from_utf8_lossy(&request.original.message.payload);
        info!("<-- payload: {data}");
    }
}

fn log_response(response: &CoapResponse) {
    let code = response.message.header.code.to_string();
    let data = String::from_utf8_lossy(&response.message.payload);
    info!("--> {code:?} {data}");
}

async fn resp_default(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let mut resp = request.new_response();
    resp.set_status(ResponseType::NotFound);
    resp.message.payload = "NOT FOUND".into();

    log_response(&resp);
    Ok(resp)
}

async fn resp_get_sensor(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let path = &request.unmatched_path;
    let mut resp = request.new_response();
    resp.set_status(ResponseType::NotFound);
    resp.message.payload = "NOT FOUND".into();

    if !path.is_empty() {
        let t = mydata().average_out_t().await;
        if let Some(d) = mydata().average_get(&path[0], t).await {
            resp.set_status(ResponseType::Content);
            resp.message.payload = format!("{d:.2}").into();
        }
    }

    log_response(&resp);
    Ok(resp)
}

async fn resp_get_avg_out(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let mut resp = request.new_response();
    match mydata().average_out().await {
        None => {
            resp.set_status(ResponseType::ServiceUnavailable);
            resp.message.payload = "NO DATA".into();
        }
        Some(avg) => {
            resp.set_status(ResponseType::Content);
            resp.message.payload = format!("{avg:.2}").into();
        }
    }

    log_response(&resp);
    Ok(resp)
}

async fn resp_get_dump(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let mut resp = request.new_response();
    mydata().dump().await;
    resp.set_status(ResponseType::Content);
    resp.message.payload = "SEE SERVER LOG".into();

    log_response(&resp);
    Ok(resp)
}

async fn resp_get_list_sensors(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let mut resp = request.new_response();
    resp.set_status(ResponseType::Content);
    resp.message.payload = mydata().sensors_list().await.join(" ").into();

    log_response(&resp);
    Ok(resp)
}

async fn resp_post_set_outsensor(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let mut resp = request.new_response();
    let payload = &String::from_utf8_lossy(&request.original.message.payload);
    match payload.len().cmp(&0) {
        Ordering::Greater => {
            resp.set_status(ResponseType::Content);
            resp.message.payload = "OK".into();
        }
        _ => {
            resp.set_status(ResponseType::BadRequest);
            resp.message.payload = "NO DATA".into();
        }
    }

    log_response(&resp);
    Ok(resp)
}

async fn resp_post_store_temp(request: Request<SocketAddr>) -> Result<Response, CoapError> {
    log_request(&request);

    let mut resp = request.new_response();
    let payload = String::from_utf8_lossy(&request.original.message.payload).into_owned();
    match payload.len().cmp(&0) {
        Ordering::Greater => {
            let indata = payload.split_whitespace().collect::<Vec<&str>>();
            if indata.len() == 2 {
                match indata[1].parse::<f32>() {
                    Ok(temp) => {
                        let mydata = mydata();
                        let name = indata[0].to_string();
                        tokio::spawn(async move {
                            mydata.add(name, temp).await;
                        });
                        resp.set_status(ResponseType::Content);
                        resp.message.payload = "OK".into();
                    }
                    Err(_) => {
                        resp.set_status(ResponseType::BadRequest);
                        resp.message.payload = "INVALID NUMBER".into();
                    }
                }
            } else {
                resp.set_status(ResponseType::BadRequest);
                resp.message.payload = "INVALID DATA".into();
            }
        }
        _ => {
            resp.set_status(ResponseType::BadRequest);
            resp.message.payload = "NO DATA".into();
        }
    }

    log_response(&resp);
    Ok(resp)
}

// EOF
