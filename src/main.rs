// main.rs
#![feature(once_cell)]
#![feature(async_closure)]
#![feature(destructuring_assignment)]

use build_timestamp::build_time;
use log::*;
use simplelog::*;

mod utils;
use utils::coapserver;
use utils::influxdb;
use utils::sensordata;

build_time!("%A %Y-%m-%d %H:%M:%S UTC");
fn main() {
    SimpleLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new()
            .set_time_format_str("%Y-%m-%d %H:%M:%S")
            .build(),
    )
    .unwrap();
    info!("CoAP server (built {}) initializing", BUILD_TIME);
    influxdb::init();
    sensordata::init();
    coapserver::init();
    coapserver::serve_coap();
}
// EOF
