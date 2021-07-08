// main.rs
#![feature(once_cell)]

use build_timestamp::build_time;
use log::*;
use simplelog::*;
use structopt::StructOpt;

mod utils;
use utils::*;

build_time!("%A %Y-%m-%d %H:%M:%S UTC");
fn main() {
    let opt = options::GlobalServerOptions::from_args();

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

    info!("CoAP server built {}", BUILD_TIME);
    trace!("Options: {:?}", opt);
    info!("Initializing...");
    influxdb::init(&opt);
    sensordata::init(&opt);
    coapserver::init(&opt);
    coapserver::serve_coap(&opt);
}
// EOF
