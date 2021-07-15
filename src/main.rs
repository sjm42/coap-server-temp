// main.rs
#![feature(once_cell)]

use log::*;
use simplelog::*;

mod utils;
use utils::{coapserver, influxdb, options::*, sensordata};

fn main() {
    let opt = GlobalServerOptions::from_args();
    let loglevel = if opt.trace {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };
    SimpleLogger::init(
        loglevel,
        ConfigBuilder::new()
            .set_time_format_str("%Y-%m-%d %H:%M:%S")
            .build(),
    )
    .unwrap();

    info!(
        "CoAP server built from branch: {} commit: {}",
        env!("GIT_BRANCH"),
        env!("GIT_COMMIT")
    );
    info!("Source timestamp: {}", env!("SOURCE_TIMESTAMP"));
    info!("Compiler version: {}", env!("RUSTC_VERSION"));
    trace!("Options: {:?}", opt);
    info!("Initializing...");
    let jh_s = sensordata::init(&opt);
    let jh_i = influxdb::init(&opt);

    // Enter CoAP server loop
    coapserver::run(&opt);

    // Normally never reached
    let res_s = jh_s.join();
    info!("Sensordata thread exit status: {:?}", res_s);
    let res_i = jh_i.join();
    info!("InfluxDB thread exit status: {:?}", res_i);
}
// EOF
