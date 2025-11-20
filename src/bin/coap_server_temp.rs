// bin/coap_server_temp.rs

use std::{
    net::SocketAddr,
    sync::{atomic, Arc},
    time,
};

use clap::Parser;
use coap_lite::{CoapResponse, RequestType, ResponseType};
use coap_server::{
    app::{self, CoapError, Request, Response},
    CoapServer,
};
use coap_server_tokio::transport::udp::UdpTransport;
use tracing::*;

use coap_server_temp::*;
use influxdb::InfluxSender;
use sensordata::MyData;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::parse();
    opts.finalize()?;
    debug!("Global config: {opts:?}");
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let srv_state = Arc::new(ServerState {
        mydata: MyData::new(&opts),
        counter: atomic::AtomicU64::new(0),
    });

    tokio::spawn(run_expire(srv_state.clone(), opts.expire_interval));
    tokio::spawn(InfluxSender::new(&opts, srv_state.clone()).run_db_send());

    let addr = opts.listen.to_string();

    let server = CoapServer::bind(UdpTransport::new(&addr)).await?;
    info!("Listening on {addr}");

    info!("Server running...");
    Ok(server
        .serve(
            app::new()
                .resource(app::resource("/avg_out").get({
                    let state = srv_state.clone();
                    move |req| resp_get_avg_out(req, state.clone())
                }))
                .resource(app::resource("/dump").get({
                    let state = srv_state.clone();
                    move |req| resp_get_dump(req, state.clone())
                }))
                .resource(app::resource("/list_sensors").get({
                    let state = srv_state.clone();
                    move |req| resp_get_list_sensors(req, state.clone())
                }))
                .resource(app::resource("/sensor").get({
                    let state = srv_state.clone();
                    move |req| resp_get_sensor(req, state.clone())
                }))
                .resource(app::resource("/set_outsensor").post({
                    let state = srv_state.clone();
                    move |req| resp_post_set_outsensor(req, state.clone())
                }))
                .resource(app::resource("/store_temp").post({
                    let state = srv_state.clone();
                    move |req| resp_post_store_temp(req, state.clone())
                }))
                .resource(app::resource("/store").post({
                    let state = srv_state.clone();
                    move |req| resp_post_store_temp(req, state.clone())
                }))
                .resource(app::resource("/").default_handler({
                    let state = srv_state.clone();
                    move |req| resp_default(req, state.clone())
                })),
        )
        .await?)
}

pub async fn run_expire(mystate: Arc<ServerState>, interval: u64) {
    loop {
        mystate.mydata.expire(interval).await;
        error!("Expire task exited, should not happen");
        tokio::time::sleep(time::Duration::new(10, 0)).await;
        error!("Restarting expire task...");
    }
}

fn log_request(request: &Request<SocketAddr>, mystate: &mut Arc<ServerState>) {
    let id = mystate.counter.fetch_add(1, atomic::Ordering::Relaxed);
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

async fn resp_get_avg_out(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let mut resp = request.new_response();
    match mystate.mydata.average_out().await {
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

async fn resp_get_dump(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let mut resp = request.new_response();
    mystate.mydata.dump().await;
    resp.set_status(ResponseType::Content);
    resp.message.payload = "SEE SERVER LOG".into();

    log_response(&resp);
    Ok(resp)
}

async fn resp_get_list_sensors(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let mut resp = request.new_response();
    resp.set_status(ResponseType::Content);
    resp.message.payload = mystate.mydata.sensors_list().await.join(" ").into();

    log_response(&resp);
    Ok(resp)
}

async fn resp_get_sensor(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let path = &request.unmatched_path;
    let mut resp = request.new_response();
    resp.set_status(ResponseType::NotFound);
    resp.message.payload = "NOT FOUND".into();

    if !path.is_empty() {
        let t = mystate.mydata.average_out_t().await;
        if let Some(d) = mystate.mydata.average_get(&path[0], t).await {
            resp.set_status(ResponseType::Content);
            resp.message.payload = format!("{d:.2}").into();
        }
    }

    log_response(&resp);
    Ok(resp)
}

async fn resp_post_set_outsensor(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let mut resp = request.new_response();
    let req_payload = &String::from_utf8_lossy(&request.original.message.payload);
    resp.message.payload = if req_payload.is_empty() {
        resp.set_status(ResponseType::BadRequest);
        "NO DATA".into()
    } else {
        mystate.mydata.set_outsensor(req_payload).await;
        resp.set_status(ResponseType::Content);
        "OK".into()
    };

    log_response(&resp);
    Ok(resp)
}

async fn resp_post_store_temp(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let mut resp = request.new_response();
    let req_payload = String::from_utf8_lossy(&request.original.message.payload);

    resp.message.payload = if req_payload.is_empty() {
        resp.set_status(ResponseType::BadRequest);
        "NO DATA".into()
    } else {
        let indata = req_payload.split_whitespace().collect::<Vec<&str>>();
        if indata.len() == 2 {
            match indata[1].parse::<f32>() {
                Ok(temp) => {
                    let name = indata[0].to_string();
                    tokio::spawn(async move {
                        mystate.mydata.add(name, temp).await;
                    });
                    resp.set_status(ResponseType::Content);
                    "OK".into()
                }
                Err(_) => {
                    resp.set_status(ResponseType::BadRequest);
                    "INVALID NUMBER".into()
                }
            }
        } else {
            resp.set_status(ResponseType::BadRequest);
            "INVALID DATA".into()
        }
    };

    log_response(&resp);
    Ok(resp)
}

async fn resp_default(
    request: Request<SocketAddr>,
    mut mystate: Arc<ServerState>,
) -> Result<Response, CoapError> {
    log_request(&request, &mut mystate);

    let mut resp = request.new_response();
    resp.set_status(ResponseType::NotFound);
    resp.message.payload = "NOT FOUND".into();

    log_response(&resp);
    Ok(resp)
}
// EOF
