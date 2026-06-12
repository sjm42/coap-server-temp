# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 project for a UDP CoAP temperature aggregation server. Library code lives in `src/lib.rs` and its modules: `config.rs` for CLI/runtime options, `sensordata.rs` for sensor state, `tbuf.rs` for rolling temperature buffers, and `influxdb.rs` for database writes. The executable entry point is `src/bin/coap_server_temp.rs`. Operational helper scripts are at the repository root: `test-drive.sh` exercises the CoAP endpoints against a running server, and `install.sh` copies the release binary into `$HOME/coap-server/bin`.

## Build, Test, and Development Commands

- `cargo build` builds the debug binary and checks normal compilation.
- `cargo build --release` builds the optimized deployment binary using the release profile in `Cargo.toml`.
- `cargo run --bin coap_server_temp -- --debug` starts the server locally with debug logging.
- `cargo test` runs unit and integration tests. There are currently no checked-in tests, so add them with logic changes.
- `cargo fmt` formats the Rust code with rustfmt defaults.
- `cargo clippy --all-targets --all-features` catches common correctness and style issues.
- `./test-drive.sh` sends sample readings with `coap-client`; run it only after starting the server and installing `coap-client`, `bc`, and `rand`.

## Coding Style & Naming Conventions

Use standard rustfmt formatting: four-space indentation, grouped imports, and idiomatic Rust naming (`snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants). Keep async shared state behind Tokio synchronization primitives as in `MyData`. Prefer `anyhow::Result` at application boundaries and explicit error logging inside long-running background tasks.

## Testing Guidelines

Place unit tests in `#[cfg(test)] mod tests` beside the module under test, especially for rolling averages, expiration behavior, payload parsing, and InfluxDB point construction. Prefer deterministic timestamps for `tbuf` tests instead of sleeping. Use integration tests only when a network-bound CoAP workflow is required. Run `cargo test`, `cargo fmt`, and `cargo clippy --all-targets --all-features` before submitting changes.

## Commit & Pull Request Guidelines

Recent history uses short, lower-case commit subjects such as `cargo update`; keep subjects concise and imperative where possible. Pull requests should describe the behavioral change, list validation commands run, mention any runtime configuration impact, and link related issues. Include logs or command output when changing CoAP endpoints, InfluxDB writes, or startup options.

## Security & Configuration Tips

Do not commit real InfluxDB tokens. The CLI defaults are development placeholders; pass production values with `--db_url`, `--token`, `--org`, and `--bucket` or an external service manager.
