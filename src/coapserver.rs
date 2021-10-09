// coapserver.rs

use super::sensordata;
use super::startup;
use super::url::*;

use coap_lite::{CoapRequest, CoapResponse, RequestType as Method, ResponseType};
use log::*;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct MyCoapServer {
    md: Arc<sensordata::MyData>,
    runtime: tokio::runtime::Runtime,
    addr: String,
    urlmap: UrlMap,
    cnt: AtomicU64,
}
impl MyCoapServer {
    pub fn new(md: Arc<sensordata::MyData>, opts: &startup::OptsCommon) -> Self {
        MyCoapServer {
            md,
            runtime: Runtime::new().unwrap(),
            addr: opts.listen.clone(),
            urlmap: UrlMap::new()
                .with_path("store_temp", Self::resp_store_temp)
                .with_path("list_sensors", Self::resp_list_sensors)
                .with_path("avg_out", Self::resp_avg_out)
                .with_path("set_outsensor", Self::resp_set_outsensor)
                .with_path("dump", Self::resp_dump),
            cnt: AtomicU64::new(0),
        }
    }
    pub fn run(&self) -> anyhow::Result<()> {
        self.runtime.block_on(async move {
            info!("Listening on {}", &self.addr);
            info!("Server running...");
            let mut server = coap::Server::new(&self.addr).unwrap();
            server
                .run(async move |req| self.handle_coap_req(req).await)
                .await
                .unwrap();
        });
        Ok(())
    }
    async fn handle_coap_req(&self, request: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
        let i = self.cnt.fetch_add(1, Ordering::Relaxed);
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
                let h = self.urlmap.get_handler(req_path);
                ret = h(&*self.md, None);
                // ret = self.urlmap.get_handler(req_path)(&mut *self.md, None);
                resp_code = ret.code();
                resp_data = ret.data();
            }
            Method::Post => {
                // Let's do relaxed UTF-8 conversion.
                let payload = &String::from_utf8_lossy(&request.message.payload);
                info!("<-- payload: {}", payload);
                // Call the URL handler with payload
                ret = self.urlmap.get_handler(req_path)(&*self.md, Some(payload));
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

    fn resp_list_sensors(md: &sensordata::MyData, _payload: Option<&str>) -> UrlResponse {
        UrlResponse::new(ResponseType::Content, md.sensors_list().join(" "))
    }

    fn resp_store_temp(md: &sensordata::MyData, payload: Option<&str>) -> UrlResponse {
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
                        md.add(indata[0], temp);
                        UrlResponse::new(ResponseType::Content, "OK")
                    }
                }
            }
        }
    }

    fn resp_avg_out(md: &sensordata::MyData, _payload: Option<&str>) -> UrlResponse {
        match md.get_avg_out() {
            None => UrlResponse::new(ResponseType::ServiceUnavailable, "NO DATA"),
            Some(avg) => UrlResponse::new(ResponseType::Content, format!("{:.2}", avg)),
        }
    }

    fn resp_set_outsensor(md: &sensordata::MyData, payload: Option<&str>) -> UrlResponse {
        match payload {
            None => UrlResponse::new(ResponseType::BadRequest, "NO DATA"),
            Some(data) => {
                md.set_outsensor(data);
                UrlResponse::new(ResponseType::Content, "OK")
            }
        }
    }

    fn resp_dump(md: &sensordata::MyData, _payload: Option<&str>) -> UrlResponse {
        md.dump();
        UrlResponse::new(ResponseType::Content, "OK")
    }
}
// EOF
