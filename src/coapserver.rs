// coapserver.rs

use super::sensordata::MyData;
use super::config::OptsCommon;
use super::url::{MyResponse, UrlMap};

use coap_lite::{CoapRequest, CoapResponse, RequestType as Method, ResponseType};
use log::*;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct MyCoapServer {
    mydata: Arc<MyData>,
    runtime: tokio::runtime::Runtime,
    addr: String,
    urlmap: UrlMap,
    counter: AtomicU64,
}

impl MyCoapServer {
    pub fn new(opts: &OptsCommon, mydata: Arc<MyData>) -> anyhow::Result<Self> {
        Ok(MyCoapServer {
            mydata,
            runtime: Runtime::new()?,
            addr: opts.listen.clone(),
            urlmap: UrlMap::new()
                .with_path("avg_out", Self::resp_avg_out)
                .with_path("dump", Self::resp_dump)
                .with_path("list_sensors", Self::resp_list_sensors)
                .with_path("set_outsensor", Self::resp_set_outsensor)
                .with_path("store_temp", Self::resp_store_temp),
            counter: AtomicU64::new(0),
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        Ok(self.runtime.block_on(async move {
            info!("Listening on {}", &self.addr);
            info!("Server running...");
            let mut server = coap::Server::new(&self.addr)?;
            server
                .run(async move |req| self.handle_coap_req(req).await)
                .await
        })?)
    }

    async fn handle_coap_req(&self, request: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
        let i = self.counter.fetch_add(1, Ordering::Relaxed);
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
        info!("#{i} {ip_str} {method:?} /{req_path}");

        match method {
            Method::Get => {
                // Call the URL handler without payload
                ret = self.urlmap.get_handler(req_path)(&self.mydata, None);
                resp_code = ret.code();
                resp_data = ret.data();
            }
            Method::Post => {
                // Let's do relaxed UTF-8 conversion.
                let payload = &String::from_utf8_lossy(&request.message.payload);
                info!("<-- payload: {payload}");
                // Call the URL handler with payload
                ret = self.urlmap.get_handler(req_path)(&self.mydata, Some(payload));
                resp_code = ret.code();
                resp_data = ret.data();
            }
            _ => {
                info!("--> Unsupported CoAP method {method:?}");
                resp_code = ResponseType::BadRequest;
                resp_data = "INVALID METHOD";
            }
        }
        info!("--> {:?} {}", resp_code, resp_data);

        match request.response {
            Some(mut message) => {
                message.set_status(resp_code);
                message.message.payload = resp_data.into();
                debug!("--> {message:?}");
                Some(message)
            }
            _ => None,
        }
    }

    fn resp_avg_out(mydata: &MyData, _payload: Option<&str>) -> MyResponse {
        match mydata.average_out() {
            None => MyResponse::new(ResponseType::ServiceUnavailable, "NO DATA"),
            Some(avg) => MyResponse::new(ResponseType::Content, format!("{avg:.2}")),
        }
    }

    fn resp_dump(mydata: &MyData, _payload: Option<&str>) -> MyResponse {
        mydata.dump();
        MyResponse::new(ResponseType::Content, "SEE LOG")
    }

    fn resp_list_sensors(mydata: &MyData, _payload: Option<&str>) -> MyResponse {
        MyResponse::new(ResponseType::Content, mydata.sensors_list().join(" "))
    }

    fn resp_set_outsensor(mydata: &MyData, payload: Option<&str>) -> MyResponse {
        match payload {
            None => MyResponse::new(ResponseType::BadRequest, "NO DATA"),
            Some(data) => {
                mydata.set_outsensor(data);
                MyResponse::new(ResponseType::Content, "OK")
            }
        }
    }

    fn resp_store_temp(mydata: &MyData, payload: Option<&str>) -> MyResponse {
        match payload {
            None => MyResponse::new(ResponseType::BadRequest, "NO DATA"),
            Some(data) => {
                let indata = data.split_whitespace().collect::<Vec<&str>>();
                if indata.len() != 2 {
                    return MyResponse::new(ResponseType::BadRequest, "INVALID DATA");
                }
                match indata[1].parse::<f32>() {
                    Err(_) => MyResponse::new(ResponseType::BadRequest, "INVALID TEMPERATURE"),
                    Ok(temp) => {
                        mydata.add(indata[0], temp);
                        MyResponse::new(ResponseType::Content, "OK")
                    }
                }
            }
        }
    }
}
// EOF
