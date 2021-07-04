// main.rs
#![feature(once_cell)]
#![feature(async_closure)]
#![feature(destructuring_assignment)]

use build_timestamp::build_time;
use log::*;
use simplelog::*;
use structopt::StructOpt;

mod utils;
use utils::*;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    trace: bool,
    #[structopt(short, long, default_value = "127.0.0.1:5683")]
    listen: String,
    #[structopt(short = "s", long, default_value = "0000000000000000")]
    out_sensor: String,
    #[structopt(long, default_value = "300")]
    avg_t_db: u64,
    #[structopt(long, default_value = "900")]
    avg_t_out: u64,
}

build_time!("%A %Y-%m-%d %H:%M:%S UTC");
fn main() {
    let opt = Opt::from_args();

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
    info!("Options: {:?}", opt);
    info!("Initializing...");
    influxdb::init();
    sensordata::init(&opt.out_sensor, &[opt.avg_t_out, opt.avg_t_db]);
    coapserver::init();
    coapserver::serve_coap(&opt.listen);
}
// EOF
