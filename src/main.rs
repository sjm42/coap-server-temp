// main.rs

use log::*;
use simplelog::*;
use std::error::Error;
mod utils;
use utils::{coapserver, influxdb, options::*, sensordata};

fn main() -> Result<(), Box<dyn Error>> {
    let opt = GlobalServerOptions::from_args();
    let loglevel = if opt.trace {
        LevelFilter::Trace
    } else if opt.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    SimpleLogger::init(
        loglevel,
        ConfigBuilder::new()
            .set_time_format_str("%Y-%m-%d %H:%M:%S")
            .build(),
    )?;

    info!("Starting CoAP server");
    debug!("Git branch: {}", env!("GIT_BRANCH"));
    debug!("Git commit: {}", env!("GIT_COMMIT"));
    debug!("Source timestamp: {}", env!("SOURCE_TIMESTAMP"));
    debug!("Compiler version: {}", env!("RUSTC_VERSION"));
    debug!("Options: {:?}", opt);
    info!("Initializing...");
    let jh_s = sensordata::init(&opt);
    let jh_i = influxdb::init(&opt);

    // Enter CoAP server loop
    coapserver::run(&opt)?;

    // Normally never reached
    let res_s = jh_s.join();
    info!("Sensordata thread exit status: {:?}", res_s);
    let res_i = jh_i.join();
    info!("InfluxDB thread exit status: {:?}", res_i);
    Ok(())
}
// EOF
