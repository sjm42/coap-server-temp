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
        let mut tbuf = Tbuf {
            averages_t: averages_t.to_vec(),
            averages: Vec::with_capacity(averages_t.len()),
            buf: Vec::with_capacity(capacity),
            buf_expire: 0,
        };
        for _a in averages_t {
            tbuf.averages.push(f64::NAN);
        }
        tbuf.buf_expire = *averages_t.iter().max().unwrap_or(&0);
        tbuf.update_averages();
        tbuf
    }

    pub fn add(&mut self, data: Tdata) -> &mut Self {
        self.buf.push(data);
        self.update_averages();
        self
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
        let too_old = SystemTime::now()
            .checked_sub(Duration::new(self.buf_expire, 0))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut n_expired = 0;

        // always leave one value
        while self.buf.len() > 1 {
            if self.buf[0].timestamp < too_old {
                n_expired += 1;
                let exp_data = self.buf.remove(0);
                trace!("Tbuf expired tdata: {exp_data:?}");
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
        let n_avgs = self.averages_t.len();

        if self.buf.is_empty() {
            // do nothing!
            // the old averages will be kept on purpose,
            //  and this is a bit ugly.
            return self;
        }

        let now = SystemTime::now();
        let mut sums = Vec::with_capacity(n_avgs);
        let mut sizes = Vec::with_capacity(n_avgs);
        let mut age_threshold = Vec::with_capacity(n_avgs);
        for i in 0..n_avgs {
            sums.push(0.0f64);
            sizes.push(0u64);
            age_threshold.push(
                now.checked_sub(Duration::new(self.averages_t[i], 0))
                    .unwrap_or(SystemTime::UNIX_EPOCH),
            );
        }

        for buf_i in 0..self.buf.len() {
            let data = self.buf[buf_i].data;
            for avg_i in 0..n_avgs {
                if self.buf[buf_i].timestamp > age_threshold[avg_i] {
                    sizes[avg_i] += 1;
                    sums[avg_i] += data;
                }
            }
        }

        for avg_i in 0..n_avgs {
            self.averages[avg_i] = match sizes[avg_i] {
                0 => {
                    if self.buf.len() == 1 {
                        // special: if there is only one value, use that and no more questions asked
                        self.buf[0].data
                    } else {
                        f64::NAN
                    }
                }
                sz => sums[avg_i] / sz as f64,
            };
        }
        self
    }
}
// EOF
