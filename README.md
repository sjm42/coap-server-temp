# coap-server-temp

A CoAP server that collects temperature sensor readings over UDP, computes rolling averages, and periodically forwards aggregated data to InfluxDB 2.0.

## Building

Requires the stable Rust toolchain.

```sh
cargo build --release
```

The release profile uses fat LTO, opt-level 3, and a single codegen unit for maximum performance.

## Usage

```sh
coap_server_temp [OPTIONS]
```

### Options

| Option | Default | Description |
|---|---|---|
| `-l, --listen` | `127.0.0.1:5683` | Server bind address |
| `--out_sensor` | `000` | Outside sensor ID(s) for `/avg_out` |
| `--average_out_t` | `900` | Outside temperature averaging window (seconds) |
| `--average_db_t` | `900` | Database averaging window (seconds) |
| `--send_interval` | `300` | InfluxDB send interval (seconds) |
| `--expire_interval` | `30` | Stale data expiration check interval (seconds) |
| `--db_url` | `http://127.0.0.1:8086` | InfluxDB URL |
| `--token` | | InfluxDB API token |
| `--org` | `myorg` | InfluxDB organization |
| `--bucket` | `temperature` | InfluxDB bucket |
| `--measurement` | `temperature` | InfluxDB measurement name |
| `-d, --debug` | | Enable debug logging |
| `-t, --trace` | | Enable trace logging |

### Example

```sh
coap_server_temp \
  --listen 0.0.0.0:5683 \
  --out_sensor 28F41A2800008091 \
  --db_url http://influxdb:8086 \
  --token my_token \
  --org my_org \
  --bucket temperature
```

## CoAP API

### Store a temperature reading

```sh
echo -n "sensor_id 21.5" | coap-client -m post -f - coap://localhost/store_temp
```

Payload format: `sensor_id temperature` (space-separated).
The `/store` path is an alias for `/store_temp`.

### Query temperatures

```sh
# Outside temperature average (based on --out_sensor)
coap-client -m get coap://localhost/avg_out

# Average for a specific sensor
coap-client -m get coap://localhost/sensor/28F41A2800008091

# List all known sensors
coap-client -m get coap://localhost/list_sensors

# Dump all sensor data to server log
coap-client -m get coap://localhost/dump
```

### Change the outside sensor at runtime

```sh
echo -n "new_sensor_id" | coap-client -m post -f - coap://localhost/set_outsensor
```

## How it works

Sensors POST readings to the server, which stores them in per-sensor circular buffers. Rolling averages are computed on the fly over configurable time windows. A background task periodically sends the aggregated averages to InfluxDB. Another background task expires stale readings to keep memory usage bounded.

## License

MIT

