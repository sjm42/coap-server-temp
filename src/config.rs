// options.rs

use log::*;
use std::env;
pub use std::ffi::OsString;
pub use structopt::StructOpt;

/// Note: internal InfluxDB client is used unless --influx-binary option is set.
#[derive(Clone, Debug, Default, StructOpt)]
pub struct OptsCommon {
    #[structopt(short, long)]
    pub debug: bool,
    #[structopt(short, long)]
    pub trace: bool,
    #[structopt(short, long, default_value = "127.0.0.1:5683")]
    pub listen: String,
    #[structopt(short = "s", long, default_value = "0000000000000000")]
    pub out_sensor: String,
    #[structopt(long, default_value = "900")]
    pub average_db_t: u64,
    #[structopt(long, default_value = "900")]
    pub average_out_t: u64,
    #[structopt(long, default_value = "300")]
    pub send_interval: i64,
    #[structopt(long, default_value = "http://127.0.0.1:8086")]
    pub db_url: String,
    #[structopt(long, default_value = "secret_token")]
    pub token: String,
    #[structopt(long, default_value = "myorg")]
    pub org: String,
    #[structopt(long, default_value = "temperature")]
    pub bucket: String,
    #[structopt(long, default_value = "temperature")]
    pub measurement: String,
    #[structopt(long, default_value = "30")]
    pub expire_interval: u64,
}
impl OptsCommon {
    pub fn finish(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    pub fn get_loglevel(&self) -> LevelFilter {
        if self.trace {
            LevelFilter::Trace
        } else if self.debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        }
    }
    pub fn start_pgm(&self, name: &str) {
        env_logger::Builder::new()
            .filter_module(env!("CARGO_PKG_NAME"), self.get_loglevel())
            .filter_module(name, self.get_loglevel())
            .filter_level(self.get_loglevel())
            .format_timestamp_secs()
            .init();
        info!("Starting up {name} v{}...", env!("CARGO_PKG_VERSION"));

        debug!("Git branch: {}", env!("GIT_BRANCH"));
        debug!("Git commit: {}", env!("GIT_COMMIT"));
        debug!("Source timestamp: {}", env!("SOURCE_TIMESTAMP"));
        debug!("Compiler version: {}", env!("RUSTC_VERSION"));
    }
}

// EOF
