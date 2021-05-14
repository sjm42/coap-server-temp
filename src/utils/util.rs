// utils/util.rs

extern crate chrono;
use chrono::prelude::*;

pub fn mylog(logentry: &String) {
    let timestamp: DateTime<Local> = Local::now();
    println!("{}: {}", timestamp.format("%Y-%m-%d %H:%M:%S"), logentry);
}

// EOF
