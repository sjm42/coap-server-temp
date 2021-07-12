// main.rs
#![feature(once_cell)]

use log::*;
use simplelog::*;

mod utils;
use utils::{coapserver, influxdb, options::*, sensordata};

fn main() {
    let opt = GlobalServerOptions::from_args();

    let loglevel = match opt.trace {
        true => LevelFilter::Trace,
        _ => LevelFilter::Info,
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
    influxdb::init(&opt);
    sensordata::init(&opt);
    coapserver::serve_coap(&opt);
}
// EOF
