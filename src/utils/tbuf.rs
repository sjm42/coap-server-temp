// tbuf.rs

use std::time::*;
use crate::utils::util::*;

#[derive(Debug)]
pub struct Tdata {
    pub ts: SystemTime,
    pub data: f32,
}

#[derive(Debug)]
pub struct Tbuf {
    pub expire: u64,
    pub avg: f64,
    pub buf: Vec<Tdata>,
}

#[allow(dead_code)]
impl Tdata {
    pub fn new(ts: SystemTime, data: f32) -> Tdata {
        Tdata {
            ts: ts,
            data: data,
        }
    }
}

#[allow(dead_code)]
impl Tbuf {
    pub fn new(t_expire: u64) -> Tbuf {
        Tbuf {
            expire: t_expire,
            avg: f64::NAN,
            buf: Vec::new(),
        }
    }
    pub fn upd_avg(&mut self) {
        let mut tsum: f64 = 0.0;
        for x in &self.buf {
            tsum += x.data as f64;
        }
        self.avg = tsum / (self.buf.len() as f64);
    }
    pub fn expire(&mut self) {
        let now = SystemTime::now();
        let expiration = now.checked_sub(Duration::from_secs(self.expire)).unwrap();
        while self.buf.len() > 0 {
            if self.buf[0].ts < expiration {
                let _exp_data = self.buf.remove(0);
                mylog(&format!("Tbuf expired tdata: {:?}", _exp_data));
            }
            else { break; }
        }
        mylog(&format!("(tbuf expire)Tbuf len: {}", self.buf.len()));
    }
    pub fn add(&mut self, d: Tdata) {
        self.buf.push(d);
        self.expire();
        self.upd_avg();
    }
}

// EOF
