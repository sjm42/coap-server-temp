// utils/tbuf.rs

use log::*;
use std::time::*;

#[derive(Debug)]
pub struct Tdata {
    ts: SystemTime,
    data: f64,
}

#[allow(dead_code)]
impl Tdata {
    pub fn new<A>(args: A) -> Tdata
    where
        A: Into<Tdata>,
    {
        args.into()
    }
    pub fn ts(&self) -> SystemTime {
        self.ts
    }
    pub fn data(&self) -> f64 {
        self.data
    }
}
impl From<f32> for Tdata {
    fn from(d: f32) -> Tdata {
        Tdata {
            ts: SystemTime::now(),
            data: d as f64,
        }
    }
}
impl From<f64> for Tdata {
    fn from(d: f64) -> Tdata {
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
            data: d as f64,
        }
    }
}
impl From<(SystemTime, f32)> for Tdata {
    fn from((t, d): (SystemTime, f32)) -> Tdata {
        Tdata {
            ts: t,
            data: d as f64,
        }
    }
}
impl From<(f64, SystemTime)> for Tdata {
    fn from((d, t): (f64, SystemTime)) -> Tdata {
        Tdata { ts: t, data: d }
    }
}
impl From<(SystemTime, f64)> for Tdata {
    fn from((t, d): (SystemTime, f64)) -> Tdata {
        Tdata { ts: t, data: d }
    }
}

#[derive(Debug)]
pub struct Tbuf {
    avgs_t: Vec<u64>,
    buf_expire: u64,
    avgs: Vec<f64>,
    buf: Vec<Tdata>,
}

#[allow(dead_code)]
impl Tbuf {
    pub fn new_avgs(expires: &[u64]) -> Tbuf {
        trace!("Tbuf::new_avgs()");
        let mut tbuf = Tbuf {
            avgs_t: expires.to_vec(),
            buf_expire: *expires.iter().max().unwrap(),
            avgs: Vec::with_capacity(expires.len()),
            buf: Vec::new(),
        };
        // Vector avgs is guaranteed to be of same length as avgs_t
        // so we are filling it up here now.
        for _a in expires.iter() {
            tbuf.avgs.push(f64::NAN);
        }
        tbuf
    }
    // Default constructor will create 5min and 15min averages (deprecated)
    pub fn new() -> Tbuf {
        trace!("Tbuf::new()");
        Tbuf::new_avgs(&[5 * 60, 15 * 60])
    }
    pub fn set_avgs(&mut self, expires: &[u64]) {
        self.avgs_t = expires.to_vec();
        self.buf_expire = *expires.iter().max().unwrap();
        self.avgs = Vec::with_capacity(expires.len());
        for _a in expires.iter() {
            self.avgs.push(f64::NAN);
        }
        self.upd_avg();
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn add(&mut self, d: Tdata) {
        trace!("Tbuf::add({:?})", d);
        self.buf.push(d);
        self.upd_avg();
    }
    pub fn avg(&self, t: u64) -> Option<f64> {
        for i in 0..self.avgs_t.len() {
            if t == self.avgs_t[i] {
                return Some(self.avgs[i]);
            }
        }
        None
    }
    pub fn avg5(&self) -> f64 {
        match self.avg(300) {
            None => f64::NAN,
            Some(a) => a,
        }
    }
    pub fn avg10(&self) -> f64 {
        match self.avg(600) {
            None => f64::NAN,
            Some(a) => a,
        }
    }
    pub fn avg15(&self) -> f64 {
        match self.avg(900) {
            None => f64::NAN,
            Some(a) => a,
        }
    }
    pub fn expire(&mut self) -> bool {
        let too_old = SystemTime::now()
            .checked_sub(Duration::from_secs(self.buf_expire))
            .unwrap();
        let mut changed = false;
        while !self.buf.is_empty() {
            if self.buf[0].ts < too_old {
                changed = true;
                let _exp_data = self.buf.remove(0);
                trace!("Tbuf expired tdata: {:?}", _exp_data);
            } else {
                // The items are age ordered and thus we stop
                // when the oldest non-expired item is found
                break;
            }
        }
        // trace!("(tbuf expire)Tbuf len: {}", self.buf.len());
        changed
    }
    pub fn upd_avg(&mut self) {
        let n_avg = self.avgs_t.len();
        // is it empty?
        if self.buf.is_empty() {
            for i in 0..n_avg {
                self.avgs[i] = f64::NAN;
            }
            return;
        }
        let now = SystemTime::now();
        let mut sums = Vec::with_capacity(n_avg);
        let mut sizes = Vec::with_capacity(n_avg);
        let mut too_old = Vec::with_capacity(n_avg);
        for i in 0..n_avg {
            sums.push(0.0f64);
            sizes.push(0u64);
            too_old.push(
                now.checked_sub(Duration::from_secs(self.avgs_t[i]))
                    .unwrap(),
            );
        }
        for buf_i in 0..self.buf.len() {
            for avg_i in 0..n_avg {
                if self.buf[buf_i].ts > too_old[avg_i] {
                    sizes[avg_i] += 1;
                    sums[avg_i] += self.buf[buf_i].data;
                }
            }
        }
        for avg_i in 0..n_avg {
            self.avgs[avg_i] = match sizes[avg_i] {
                0 => f64::NAN,
                sz => sums[avg_i] / sz as f64,
            };
        }
    }
}
// EOF
