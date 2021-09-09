// options.rs

pub use std::path::PathBuf;
pub use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// Note: internal InfluxDB client is used unless --influx-binary option is set.
pub struct GlobalServerOptions {
    #[structopt(short, long)]
    pub debug: bool,
    #[structopt(short, long)]
    pub trace: bool,
    #[structopt(short, long, default_value = "127.0.0.1:5683")]
    pub listen: String,
    #[structopt(short = "s", long, default_value = "0000000000000000")]
    pub out_sensor: String,
    #[structopt(long, default_value = "300")]
    pub avg_t_db: u64,
    #[structopt(long, default_value = "900")]
    pub avg_t_out: u64,
    #[structopt(long, default_value = "60")]
    pub send_interval: i64,
    #[structopt(long, parse(from_os_str))]
    pub influx_binary: Option<PathBuf>,
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
// EOF
