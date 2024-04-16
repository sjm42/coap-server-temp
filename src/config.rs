// options.rs

use std::env;
pub use std::ffi::OsString;

use clap::Parser;
use tracing::*;

#[derive(Clone, Debug, Default, Parser)]
pub struct OptsCommon {
    #[arg(short, long)]
    pub debug: bool,
    #[arg(short, long)]
    pub trace: bool,
    #[arg(short, long, default_value = "127.0.0.1:5683")]
    pub listen: String,
    #[arg(long, default_value = "000")]
    pub out_sensor: String,
    #[arg(long, default_value_t = 900)]
    pub average_db_t: u64,
    #[arg(long, default_value_t = 900)]
    pub average_out_t: u64,
    #[arg(long, default_value_t = 300)]
    pub send_interval: i64,
    #[arg(long, default_value = "http://127.0.0.1:8086")]
    pub db_url: String,
    #[arg(long, default_value = "secret_token")]
    pub token: String,
    #[arg(long, default_value = "myorg")]
    pub org: String,
    #[arg(long, default_value = "temperature")]
    pub bucket: String,
    #[arg(long, default_value = "temperature")]
    pub measurement: String,
    #[arg(long, default_value_t = 30)]
    pub expire_interval: u64,
}

impl OptsCommon {
    pub fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn get_loglevel(&self) -> Level {
        if self.trace {
            Level::TRACE
        } else if self.debug {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }
    pub fn start_pgm(&self, name: &str) {
        tracing_subscriber::fmt()
            .with_max_level(self.get_loglevel())
            .with_target(false)
            .init();

        info!("Starting up {name} v{}...", env!("CARGO_PKG_VERSION"));
        debug!("Git branch: {}", env!("GIT_BRANCH"));
        debug!("Git commit: {}", env!("GIT_COMMIT"));
        debug!("Source timestamp: {}", env!("SOURCE_TIMESTAMP"));
        debug!("Compiler version: {}", env!("RUSTC_VERSION"));
    }
}

// EOF
