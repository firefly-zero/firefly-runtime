use firefly_hal::{Duration, Instant};
use firefly_types::serial;

// One kilobyte.
const KB: usize = 1024;

/// How often (in update cycles) the stats should be emitted.
const FREQ: u32 = 60;

pub(crate) struct StatsTracker {
    pub frame: u32,

    /// Fuel spendings for the `update` callback.
    pub update_fuel: CallbackFuel,

    /// Fuel spendings for the `render` callback.
    pub render_fuel: CallbackFuel,

    /// Time when the CPU values were synced for the last time.
    pub synced: Instant,

    /// Time spent sleeping.
    pub delays: Duration,

    /// Time lagging behind desired FPS because of updates.
    pub lags: Duration,

    pub pages: u16,
    pub last_one: u32,
}

impl StatsTracker {
    pub fn new(now: Instant) -> Self {
        Self {
            frame: 0,
            update_fuel: CallbackFuel::default(),
            render_fuel: CallbackFuel::default(),
            synced: now,
            delays: Duration::from_ms(0),
            lags: Duration::from_ms(0),
            pages: 0,
            last_one: 0,
        }
    }

    pub fn analyze_memory(&mut self, data: &[u8]) {
        let pages = data.len() / (64 * KB);
        self.pages = pages as u16;
        if self.frame % FREQ != 10 {
            return;
        }
        for (i, byte) in data.iter().rev().enumerate() {
            if byte != &0 {
                self.last_one = (data.len() - i) as u32;
                return;
            }
        }
    }

    pub fn as_message(&mut self, now: Instant) -> Option<serial::Response> {
        self.frame = self.frame.wrapping_add(1);
        // Skip the first period, we don't have enough stats yet.
        if self.frame < FREQ {
            return None;
        };
        let message = match self.frame % FREQ {
            3 => {
                let cpu = self.as_cpu(now);
                self.delays = Duration::from_ms(0);
                self.lags = Duration::from_ms(0);
                self.synced = now;
                serial::Response::CPU(cpu)
            }
            5 => {
                let fuel = self.update_fuel.as_fuel();
                self.update_fuel.reset();
                serial::Response::Fuel(serial::Callback::Update, fuel)
            }
            7 => {
                let fuel = self.render_fuel.as_fuel();
                self.render_fuel.reset();
                serial::Response::Fuel(serial::Callback::Render, fuel)
            }
            12 => {
                let memory = serial::Memory {
                    pages: self.pages,
                    last_one: self.last_one,
                    reads: 0,
                    writes: 0,
                    max: 0,
                };
                serial::Response::Memory(memory)
            }
            _ => return None,
        };
        Some(message)
    }

    fn as_cpu(&self, now: Instant) -> serial::CPU {
        let total = now - self.synced;
        let busy = total.ns().saturating_sub(self.delays.ns());
        serial::CPU {
            busy_ns: self.lags.ns().saturating_add(busy),
            lag_ns: self.lags.ns(),
            total_ns: total.ns(),
        }
    }
}

#[derive(Default)]
pub(crate) struct CallbackFuel {
    min: Option<u32>,
    max: u32,
    sum: u32,
    mean: f32,
    m2: f32,
    count: u32,
}

impl CallbackFuel {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn add(&mut self, v: u32) {
        self.min = match self.min {
            Some(min) => Some(min.min(v)),
            None => Some(v),
        };
        self.max = self.max.max(v);
        self.sum += v;
        self.count += 1;

        // https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
        let v = v as f32;
        let delta = v - self.mean;
        self.mean += delta / self.count as f32;
        let delta2 = v - self.mean;
        self.m2 += delta * delta2;
    }

    fn as_fuel(&self) -> serial::Fuel {
        let v = self;
        if v.count == 0 {
            return serial::Fuel {
                min: 0,
                max: 0,
                mean: 0,
                var: 0.,
                calls: 0,
            };
        }
        let m2 = if v.count <= 1 { 0.0 } else { v.m2 };
        serial::Fuel {
            min: v.min.unwrap_or_default(),
            max: v.max,
            mean: v.sum / v.count,
            var: m2 / v.count as f32,
            calls: v.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel() {
        let mut fuel = CallbackFuel::default();
        for i in [2, 4, 4, 4, 5, 5, 7, 9] {
            fuel.add(i);
        }
        let fuel: serial::Fuel = fuel.as_fuel();
        assert_eq!(fuel.calls, 8);
        assert_eq!(fuel.min, 2);
        assert_eq!(fuel.max, 9);
        assert_eq!(fuel.mean, 5);
        assert_eq!(fuel.var, 4.);
    }
}
