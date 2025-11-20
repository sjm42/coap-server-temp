// lib.rs

use crate::sensordata::MyData;
pub use config::*;
use std::sync::atomic;

pub mod config;

pub mod influxdb;
pub mod sensordata;
pub mod tbuf;

pub struct ServerState {
    pub mydata: MyData,
    pub counter: atomic::AtomicU64,
}

// EOF
