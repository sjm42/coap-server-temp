// influxdb.rs

use super::sensordata;
use super::startup;

use anyhow::anyhow;
use chrono::*;
use log::*;
use std::{ffi::OsString, io::Write, process::*, sync::Arc, thread, time};
use tokio::runtime::Runtime;

#[derive(Clone, Debug)]
struct InfluxCtx {
    interval: i64,
    binary: Option<OsString>,
    url: String,
    token: String,
    org: String,
    bucket: String,
    measurement: String,
}

pub fn init(md: Arc<sensordata::MyData>, opt: &startup::OptsCommon) {
    trace!("influxdb::init()");
    let ctx = InfluxCtx {
        interval: opt.send_interval,
        binary: opt.influx_binary.clone(),
        url: opt.db_url.clone(),
        token: opt.token.clone(),
        org: opt.org.clone(),
        bucket: opt.bucket.clone(),
        measurement: opt.measurement.clone(),
    };
    // Start a new background thread for database inserts
    thread::spawn(move || {
        run_db_send(md, ctx);
    });
}

fn run_db_send(md: Arc<sensordata::MyData>, ctx: InfluxCtx) {
    loop {
        let md_i = md.clone();
        let ctx_i = ctx.clone();

        let jh = thread::spawn(move || match ctx_i.binary {
            Some(ref bin) => {
                info!("Using external Influx binary {:?}", bin);
                db_send_external(md_i, ctx_i)
            }
            None => {
                info!("Using the internal InfluxDB client");
                db_send_internal(md_i, ctx_i)
            }
        });
        debug!(
            "InfluxDB data push thread started as id {:?}",
            jh.thread().id()
        );
        // We are blocking in join() until child thread exits -- should never happen.
        let res = jh.join();
        error!("InfluxDB thread exited! Reason: {:?}", res);
        thread::sleep(time::Duration::new(10, 0));
        error!("Restarting InfluxDB thread...");
    }
}

// Use the Rust native influxdb client library
fn db_send_internal(md: Arc<sensordata::MyData>, ctx: InfluxCtx) {
    let rt = Runtime::new().unwrap();
    loop {
        let waitsec = ctx.interval - (Utc::now().timestamp() % ctx.interval);
        // wait until next interval start
        thread::sleep(time::Duration::new(waitsec as u64, 0));

        trace!("influxdb::db_send_internal() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % ctx.interval);

        let mut pts = Vec::with_capacity(8);
        for datapoint in md.sensors_db() {
            pts.push(
                influxdb_client::Point::new(&ctx.measurement)
                    .tag("sensor", datapoint.0.as_str())
                    .field("value", datapoint.1)
                    .timestamp(ts_i),
            );
        }
        if !pts.is_empty() {
            let c = influxdb_client::Client::new(&ctx.url, &ctx.token)
                .with_org(&ctx.org)
                .with_bucket(&ctx.bucket)
                .with_precision(influxdb_client::Precision::S);

            rt.spawn(async move {
                // ownership of pts and c are moved into here
                // hence, new ones must be created each time before calling this
                debug!("influxdb data: {:?}", &pts);
                match c
                    .insert_points(&pts, influxdb_client::TimestampOptions::FromPoint)
                    .await
                {
                    Ok(_) => {
                        info!("****** InfluxDB: inserted {} points", pts.len());
                    }
                    Err(e) => {
                        error!("InfluxDB client error: {:?}", e);
                    }
                }
            });
        }
    }
}

fn db_send_external(md: Arc<sensordata::MyData>, ctx: InfluxCtx) {
    let mut data_points = Vec::with_capacity(8);
    loop {
        let waitsec = ctx.interval - (Utc::now().timestamp() % ctx.interval);
        // wait until next interval start
        thread::sleep(time::Duration::new(waitsec as u64, 0));

        trace!("influxdb::db_send_ext() active");
        let ts = Utc::now().timestamp();
        let ts_i = ts - (ts % ctx.interval);

        data_points.clear();
        for datapoint in md.sensors_db() {
            data_points.push(format!(
                "{},sensor={} value={:.2} {}\n",
                ctx.measurement, &datapoint.0, datapoint.1, ts_i
            ));
        }
        if !data_points.is_empty() {
            match influx_run_cmd(&data_points, &ctx) {
                Ok(_) => {
                    info!("****** InfluxDB: inserted {} points", data_points.len());
                }
                Err(e) => {
                    error!("InfluxDB client error: {:?}", e);
                }
            }
        }
    }
}

fn influx_run_cmd(data_points: &[String], ctx: &InfluxCtx) -> anyhow::Result<()> {
    let line_data = data_points.join("\n");
    let iargs = [
        "write",
        "--precision",
        "s",
        "--host",
        &ctx.url,
        "--token",
        &ctx.token,
        "--org",
        &ctx.org,
        "--bucket",
        &ctx.bucket,
    ];
    trace!("Running {:?} {}", &ctx.binary, iargs.join(" "));
    debug!("data:\n{}", line_data);
    let mut p = Command::new(ctx.binary.as_ref().unwrap())
        .args(&iargs)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let p_in = p.stdin.as_mut().unwrap();
    p_in.write_all(line_data.as_bytes()).unwrap();
    let out = p.wait_with_output().unwrap();
    if out.status.success() && out.stdout.is_empty() && out.stderr.is_empty() {
        Ok(())
    } else {
        error!(
            "influx command failed, exit status {}\nstderr:\n{}\nstdout:\n{}\n",
            out.status.code().unwrap(),
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        Err(anyhow!(out.status))
    }
}
// EOF
