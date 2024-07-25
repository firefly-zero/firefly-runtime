pub(crate) struct StatsTracker {
    /// Fuel spendings for the `update` callback.
    pub update_fuel: CallbackFuel,

    /// Fuel spendings for the `render` callback.
    pub render_fuel: CallbackFuel,

    /// Time when the CPU values were synced for the last time.
    pub synced: firefly_device::Instant,

    /// Time spent sleeping.
    pub delays: firefly_device::Duration,

    /// Time lagging behind desired FPS because of updates.
    pub lags: firefly_device::Duration,
}

impl StatsTracker {
    fn as_cpu(&mut self, now: firefly_device::Instant) -> firefly_types::serial::CPU {
        let total = now - self.synced;
        self.synced = now;
        firefly_types::serial::CPU {
            busy_ns: self.lags.ns() + (total.ns() - self.delays.ns()),
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
        self.m2 += delta * delta2
    }
}

impl From<CallbackFuel> for firefly_types::serial::Fuel {
    fn from(v: CallbackFuel) -> Self {
        if v.count == 0 {
            return Self {
                min: 0,
                max: 0,
                mean: 0,
                var: 0.,
                calls: 0,
            };
        }
        let m2 = if v.count <= 1 { 0.0 } else { v.m2 };
        Self {
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
        for i in vec![2, 4, 4, 4, 5, 5, 7, 9] {
            fuel.add(i);
        }
        let fuel: firefly_types::serial::Fuel = fuel.into();
        assert_eq!(fuel.calls, 8);
        assert_eq!(fuel.min, 2);
        assert_eq!(fuel.max, 9);
        assert_eq!(fuel.mean, 5);
        assert_eq!(fuel.var, 4.);
    }
}
