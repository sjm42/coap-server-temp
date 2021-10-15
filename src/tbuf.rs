// tbuf.rs

use log::*;
use std::time::*;

#[derive(Debug)]
pub struct Tdata {
    timestamp: SystemTime,
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
        self.timestamp
    }
    pub fn data(&self) -> f64 {
        self.data
    }
}
impl From<f32> for Tdata {
    fn from(d: f32) -> Tdata {
        Tdata {
            timestamp: SystemTime::now(),
            data: d as f64,
        }
    }
}
impl From<f64> for Tdata {
    fn from(d: f64) -> Tdata {
        Tdata {
            timestamp: SystemTime::now(),
            data: d,
        }
    }
}
impl From<(f32, SystemTime)> for Tdata {
    fn from((d, t): (f32, SystemTime)) -> Tdata {
        Tdata {
            timestamp: t,
            data: d as f64,
        }
    }
}
impl From<(SystemTime, f32)> for Tdata {
    fn from((t, d): (SystemTime, f32)) -> Tdata {
        Tdata {
            timestamp: t,
            data: d as f64,
        }
    }
}
impl From<(f64, SystemTime)> for Tdata {
    fn from((d, t): (f64, SystemTime)) -> Tdata {
        Tdata {
            timestamp: t,
            data: d,
        }
    }
}
impl From<(SystemTime, f64)> for Tdata {
    fn from((t, d): (SystemTime, f64)) -> Tdata {
        Tdata {
            timestamp: t,
            data: d,
        }
    }
}

#[derive(Debug)]
pub struct Tbuf {
    averages_t: Vec<u64>,
    averages: Vec<f64>,
    buf: Vec<Tdata>,
    buf_expire: u64,
}

#[allow(dead_code)]
impl Tbuf {
    pub fn new(averages_t: &[u64]) -> Tbuf {
        Tbuf::with_capacity(64, averages_t)
    }
    pub fn with_capacity(capacity: usize, averages_t: &[u64]) -> Tbuf {
        trace!("Tbuf::new_cap({}, {:?})", capacity, averages_t);
        let tbuf = Tbuf {
            averages_t: Vec::new(),
            averages: Vec::new(),
            buf: Vec::with_capacity(capacity),
            buf_expire: 0,
        };
        tbuf.with_averages(averages_t)
    }
    pub fn with_averages(mut self, averages_t: &[u64]) -> Self {
        trace!("Tbuf::with_avgs({:?})", averages_t);
        self.averages_t = averages_t.to_vec();
        self.averages = Vec::with_capacity(averages_t.len());
        for _a in averages_t {
            self.averages.push(f64::NAN);
        }
        self.buf_expire = *averages_t.iter().max().unwrap();
        self.update_averages();
        self
    }
    pub fn add(&mut self, data: Tdata) -> &mut Self {
        trace!("Tbuf::add({:?})", data);
        self.buf.push(data);
        self.update_averages();
        self
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn average(&self, time_sec: u64) -> Option<f64> {
        for i in 0..self.averages_t.len() {
            if time_sec == self.averages_t[i] {
                return Some(self.averages[i]);
            }
        }
        None
    }
    pub fn expire(&mut self) -> usize {
        trace!("Tbuf::expire()");
        let too_old = SystemTime::now()
            .checked_sub(Duration::new(self.buf_expire, 0))
            .unwrap();
        let mut n_expired = 0;
        while !self.buf.is_empty() {
            if self.buf[0].timestamp < too_old {
                n_expired += 1;
                let _exp_data = self.buf.remove(0);
                trace!("Tbuf expired tdata: {:?}", _exp_data);
            } else {
                // The items are age ordered and thus we stop
                // when the oldest non-expired item is found
                break;
            }
        }
        // trace!("(tbuf expire)Tbuf len: {}", self.buf.len());
        n_expired
    }
    pub fn update_averages(&mut self) -> &mut Self {
        trace!("Tbuf::update_avgs()");
        let n_avgs = self.averages_t.len();
        // is it empty?
        if self.buf.is_empty() {
            for i in 0..n_avgs {
                self.averages[i] = f64::NAN;
            }
            return self;
        }

        let now = SystemTime::now();
        let mut sums = Vec::with_capacity(n_avgs);
        let mut sizes = Vec::with_capacity(n_avgs);
        let mut too_old = Vec::with_capacity(n_avgs);
        for i in 0..n_avgs {
            sums.push(0.0f64);
            sizes.push(0u64);
            too_old.push(
                now.checked_sub(Duration::new(self.averages_t[i], 0))
                    .unwrap(),
            );
        }
        for buf_i in 0..self.buf.len() {
            let data = self.buf[buf_i].data;
            for avg_i in 0..n_avgs {
                if self.buf[buf_i].timestamp > too_old[avg_i] {
                    sizes[avg_i] += 1;
                    sums[avg_i] += data;
                }
            }
        }
        for avg_i in 0..n_avgs {
            self.averages[avg_i] = match sizes[avg_i] {
                0 => f64::NAN,
                sz => sums[avg_i] / sz as f64,
            };
        }
        self
    }
}
// EOF
