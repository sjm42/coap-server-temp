# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CoAP server for collecting temperature sensor data over UDP and forwarding aggregated readings to InfluxDB 2.0.
Single-crate Rust project (edition 2024) with one binary target (`coap_server_temp`).

## Build Commands

- `cargo build` — debug build
- `cargo build --release` — release build (fat LTO, opt-level 3, codegen-units 1)
- `cargo check` — quick compilation check
- `cargo clippy` — lint
- `cargo fmt --check` — format check
- `cargo run --bin coap_server_temp -- [OPTIONS]` — run the server

There are no unit tests. Integration testing is done manually via `test-drive.sh` using `coap-client`.

## Architecture

**ServerState** (`lib.rs`) is the shared application state (`Arc<ServerState>`) holding sensor data and a request
counter.

**Data flow:** CoAP clients POST temperature readings → `MyData` stores per-sensor data in `Tbuf` ring buffers →
background tasks periodically expire stale data and send aggregates to InfluxDB.

### Key modules

- **`config.rs`** — CLI argument parsing via clap derive. All server configuration (listen address, InfluxDB
  credentials, averaging windows, intervals).
- **`sensordata.rs`** — `MyData`: thread-safe sensor registry (`RwLock<HashMap<String, Tbuf>>`). Handles
  add/average/expire/dump operations.
- **`tbuf.rs`** — `Tbuf`: circular buffer of `Tdata` (value + timestamp) with configurable rolling average windows.
  `Tdata` has `From` impls for f32/f64 with optional timestamps.
- **`influxdb.rs`** — `InfluxSender`: background task that periodically sends aggregated sensor averages to InfluxDB
  2.0, synchronized to interval boundaries.
- **`bin/coap_server_temp.rs`** — Entry point. Sets up CoAP server with UDP transport, spawns expire and DB-send
  background tasks, defines all CoAP endpoint handlers.

### CoAP endpoints

| Method | Path                    | Description                                         |
|--------|-------------------------|-----------------------------------------------------|
| POST   | `/store_temp`, `/store` | Store reading: payload is `"sensor_id temperature"` |
| POST   | `/set_outsensor`        | Set outside sensor ID(s)                            |
| GET    | `/avg_out`              | Outside temperature average                         |
| GET    | `/sensor/{id}`          | Average for specific sensor                         |
| GET    | `/list_sensors`         | List all sensor IDs                                 |
| GET    | `/dump`                 | Dump data to server log                             |

### Concurrency patterns

- `Arc<ServerState>` shared across all async handlers and background tasks
- `RwLock` for sensor data map, `AtomicU64` for request counter
- Background tasks (`tokio::spawn`) for data expiration and InfluxDB sends, with automatic restart on failure

## Dependencies of note

- `coap-server` and `coap-server-tokio` are git dependencies from `github.com/jasta/coap-server-rs`
- `coap-lite` pinned to 0.9 for compatibility with coap-server
- `build-data` build dependency injects git metadata (branch, commit, timestamp) at compile time via `build.rs`
