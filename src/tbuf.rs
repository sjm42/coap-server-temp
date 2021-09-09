// tbuf.rs

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
    buf_expire: u64,
    avgs_t: Vec<u64>,
    avgs: Vec<f64>,
    buf: Vec<Tdata>,
}

#[allow(dead_code)]
impl Tbuf {
    pub fn new(avgs_t: &[u64]) -> Tbuf {
        Tbuf::new_cap(16, avgs_t)
    }
    pub fn new_cap(cap: usize, avgs_t: &[u64]) -> Tbuf {
        trace!("Tbuf::new_cap({}, {:?})", cap, avgs_t);
        let mut tbuf = Tbuf {
            buf_expire: *avgs_t.iter().max().unwrap(),
            avgs_t: avgs_t.to_vec(),
            avgs: Vec::with_capacity(avgs_t.len()),
            buf: Vec::with_capacity(cap),
        };
        // Vector avgs is guaranteed to be of same length as avgs_t
        // so we are filling it up here now.
        for _a in avgs_t.iter() {
            tbuf.avgs.push(f64::NAN);
        }
        tbuf
    }
    pub fn with_avgs(&mut self, avgs_t: &[u64]) -> &mut Self {
        trace!("Tbuf::with_avgs({:?})", avgs_t);
        self.buf_expire = *avgs_t.iter().max().unwrap();
        self.avgs_t = avgs_t.to_vec();
        self.avgs = Vec::with_capacity(avgs_t.len());
        for _a in avgs_t.iter() {
            self.avgs.push(f64::NAN);
        }
        self.update_avgs();
        self
    }
    pub fn add(&mut self, d: Tdata) -> &mut Self {
        trace!("Tbuf::add({:?})", d);
        self.buf.push(d);
        self.update_avgs();
        self
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn avg(&self, t: u64) -> Option<f64> {
        for i in 0..self.avgs_t.len() {
            if t == self.avgs_t[i] {
                return Some(self.avgs[i]);
            }
        }
        None
    }
    pub fn expire(&mut self) -> usize {
        trace!("Tbuf::expire()");
        let too_old = SystemTime::now()
            .checked_sub(Duration::new(self.buf_expire, 0))
            .unwrap();
        let mut n_exp = 0;
        while !self.buf.is_empty() {
            if self.buf[0].ts < too_old {
                n_exp += 1;
                let _exp_data = self.buf.remove(0);
                trace!("Tbuf expired tdata: {:?}", _exp_data);
            } else {
                // The items are age ordered and thus we stop
                // when the oldest non-expired item is found
                break;
            }
        }
        // trace!("(tbuf expire)Tbuf len: {}", self.buf.len());
        n_exp
    }
    pub fn update_avgs(&mut self) -> &mut Self {
        trace!("Tbuf::update_avgs()");
        let n_avg = self.avgs_t.len();
        // is it empty?
        if self.buf.is_empty() {
            for i in 0..n_avg {
                self.avgs[i] = f64::NAN;
            }
            return self;
        }

        let now = SystemTime::now();
        let mut sums = Vec::with_capacity(n_avg);
        let mut sizes = Vec::with_capacity(n_avg);
        let mut too_old = Vec::with_capacity(n_avg);
        for i in 0..n_avg {
            sums.push(0.0f64);
            sizes.push(0u64);
            too_old.push(now.checked_sub(Duration::new(self.avgs_t[i], 0)).unwrap());
        }
        for buf_i in 0..self.buf.len() {
            let data = self.buf[buf_i].data;
            for avg_i in 0..n_avg {
                if self.buf[buf_i].ts > too_old[avg_i] {
                    sizes[avg_i] += 1;
                    sums[avg_i] += data;
                }
            }
        }
        for avg_i in 0..n_avg {
            self.avgs[avg_i] = match sizes[avg_i] {
                0 => f64::NAN,
                sz => sums[avg_i] / sz as f64,
            };
        }
        self
    }
}
// EOF
