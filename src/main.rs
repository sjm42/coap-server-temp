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

use coapserver::MyCoapServer;
use influxdb::InfluxCtx;
use sensordata::*;
use startup::*;

fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    start_pgm(&opts, "CoAP server");
    opts.finish()?;
    debug!("Global config: {:?}", &opts);

    let md = Arc::new(MyData::new(&opts));
    start_expire(md.clone(), &opts);

    let idb = InfluxCtx::new(&opts, md.clone());
    idb.start_db_send();

    // Enter CoAP server loop
    let srv = MyCoapServer::new(md, &opts);
    srv.run()?;

    // Normally never reached
    Ok(())
}
// EOF
