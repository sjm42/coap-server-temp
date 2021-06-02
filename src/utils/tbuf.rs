// utils/tbuf.rs

use std::time::*;
use log::*;


#[derive(Debug)]
pub struct Tdata {
    ts: SystemTime,
    data: f32,
}

#[allow(dead_code)]
impl Tdata {
    pub fn new<A>(args: A) -> Tdata
        where A: Into<Tdata>
    {
        args.into()
    }
    pub fn ts(&self) -> SystemTime {
        self.ts
    }
    pub fn data(&self) -> f32 {
        self.data
    }
}
impl From<f32> for Tdata {
    fn from(d: f32) -> Tdata {
        Tdata {
            ts: SystemTime::now(),
            data: d,
        }
    }

}
impl From<(f32, SystemTime)> for Tdata {
    fn from((d, t): (f32, SystemTime)) -> Tdata {
        Tdata {
            ts: t,
            data: d,
        }
    }
}
impl From<(SystemTime, f32)> for Tdata {
    fn from((t, d): (SystemTime, f32)) -> Tdata {
        Tdata {
            ts: t,
            data: d,
        }
    }
}

#[derive(Debug)]
pub struct Tbuf {
    expire: u64,
    avg5: f32,
    avg15: f32,
    buf: Vec<Tdata>,
}

impl Tbuf {
    pub fn new() -> Tbuf {
        trace!("Tbuf::new()");
        Tbuf {
            expire: 15*60, // store 15 minutes worth of data
            avg5: f32::NAN,
            avg15: f32::NAN,
            buf: Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn add(&mut self, d: Tdata) {
        trace!("Tbuf::add({:?})", d);
        self.buf.push(d);
        self.upd_avg();
    }
    pub fn avg5(&self) -> f32 {
        self.avg5
    }
    pub fn avg15(&self) -> f32 {
        self.avg15
    }
    pub fn expire(&mut self) -> bool {
        let now = SystemTime::now();
        let expiration = now.checked_sub(Duration::from_secs(self.expire)).unwrap();
        let mut changed = false;
        while self.buf.len() > 0 {
            if self.buf[0].ts < expiration {
                changed = true;
                let _exp_data = self.buf.remove(0);
                trace!("Tbuf expired tdata: {:?}", _exp_data);
            }
            else {
                // The items are age ordered and thus we stop
                // when the oldest non-expired item is found
                break;
            }
        }
        // trace!("(tbuf expire)Tbuf len: {}", self.buf.len());
        changed
    }
    pub fn upd_avg(&mut self) {
        let mut n5: u32 = 0;
        let mut sum5: f32 = 0.0;
        let mut n15: u32 = 0;
        let mut sum15: f32 = 0.0;

        // is it empty?
        if self.buf.len() == 0 {
            self.avg5 = f32::NAN;
            self.avg15 = f32::NAN;
            return;
        }

        // create 5min and 15min expiration times
        let now = SystemTime::now();
        let exp5 = now.checked_sub(Duration::from_secs(5*60)).unwrap();
        let exp15 = now.checked_sub(Duration::from_secs(15*60)).unwrap();

        for i in (0..self.buf.len()).rev() {
            if self.buf[i].ts >= exp5 {
                n5 += 1;
                sum5 += self.buf[i].data;
            }
            if self.buf[i].ts >= exp15 {
                n15 += 1;
                sum15 += self.buf[i].data;
            }
        }
        self.avg5 = sum5 / n5 as f32;
        self.avg15 = sum15 / n15 as f32;
    }
}
// EOF
