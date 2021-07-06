// utils/options.rs
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// Note: internal InfluxDB client is used unless --influx-binary option is set.
pub struct CoapServerOpts {
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
    #[structopt(long, default_value="<none>")]
    pub influx_binary: String,
    #[structopt(long, default_value = "60")]
    pub influxdb_interval: i64,
    #[structopt(long, default_value = "secret_token")]
    pub influxdb_token: String,
    #[structopt(long, default_value = "myorg")]
    pub influxdb_org: String,
    #[structopt(long, default_value = "temperature")]
    pub influxdb_bucket: String,
    #[structopt(long, default_value = "temperature")]
    pub influxdb_measurement: String,
    #[structopt(long, default_value = "http://127.0.0.1:8086")]
    pub influxdb_url: String,
}
// EOF
