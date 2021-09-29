// main.rs

use log::*;

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

    let jh_s = sensordata::init(&opts);
    let jh_i = influxdb::init(&opts);

    // Enter CoAP server loop
    coapserver::run(&opts)?;

    // Normally never reached
    let res_s = jh_s.join();
    info!("Sensordata thread exit status: {:?}", res_s);
    let res_i = jh_i.join();
    info!("InfluxDB thread exit status: {:?}", res_i);
    Ok(())
}
// EOF
