// main.rs
#![feature(async_closure)]

use log::*;
use std::sync::Arc;

mod coapserver;
mod influxdb;
mod sensordata;
mod config;
mod tbuf;
mod url;

use coapserver::MyCoapServer;
use influxdb::InfluxSender;
use sensordata::{start_expire, MyData};
use config::*;

fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    debug!("Global config: {opts:?}");

    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let mydata = Arc::new(MyData::new(&opts));
    start_expire(mydata.clone(), &opts);

    let sender = InfluxSender::new(&opts, mydata.clone());
    sender.start_db_send();

    // Enter CoAP server loop
    let server = MyCoapServer::new(&opts, mydata)?;
    server.run()?;

    // Normally never reached
    Ok(())
}
// EOF
