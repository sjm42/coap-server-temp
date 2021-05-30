// main.rs
#![feature(once_cell)]

extern crate simplelog;

use log::*;
use simplelog::*;
use coap_server_temp::*;
mod utils;


fn main() {
    SimpleLogger::init(LevelFilter::Info,
                       ConfigBuilder::new()
                           .set_time_format_str("%Y-%m-%d %H:%M:%S")
                           .build()).unwrap();
    initialize();
    serve_coap();
}
// EOF
