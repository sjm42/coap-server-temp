// main.rs

#![feature(async_closure)]

use log::*;
use std::sync::Arc;

mod coapserver;
mod influxdb;
mod sensordata;
mod startup;
mod tbuf;
mod url;

use startup::*;

fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    start_pgm(&opts, "CoAP server");
    opts.finish()?;
    debug!("Global config: {:?}", &opts);

    let md = Arc::new(sensordata::MyData::new(&opts));
    sensordata::MyData::start_expire(md.clone(), &opts);
    influxdb::init(md.clone(), &opts);

    // Enter CoAP server loop
    let srv = coapserver::MyCoapServer::new(md, &opts);
    srv.run()?;

    // Normally never reached
    Ok(())
}
// EOF
