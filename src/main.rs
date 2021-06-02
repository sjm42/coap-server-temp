// main.rs
#![feature(once_cell)]
#![feature(async_closure)]

extern crate log;
extern crate simplelog;
extern crate coap;
extern crate chrono;
extern crate influxdb_client;

use log::*;
use simplelog::*;

mod utils;
use utils::influxdb;
use utils::outsensor;
use utils::sensordata;
use utils::coapserver;


fn main() {
    SimpleLogger::init(LevelFilter::Info,
                       ConfigBuilder::new()
                           .set_time_format_str("%Y-%m-%d %H:%M:%S")
                           .build()).unwrap();
    info!("CoAP server initializing");
    influxdb::init();
    outsensor::init();
    sensordata::init();
    coapserver::init();
    coapserver::serve_coap();
}
// EOF
