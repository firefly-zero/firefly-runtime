use crate::utils::read_all;
use embedded_io::Write;
use firefly_hal::{Device, DeviceImpl, Dir, FSError};
use firefly_types::{BatteryInfo, Encode};

const K: f32 = 12.;

/// Battery status manager.
///
/// The State-of-Charge (SOC) of a battery is calculated from the current voltage
/// as a sigmoid function on the range from minimum to maximum voltage of the battery.
/// The minimum and maximum are stored in the FS, initially set to the defaults,
/// and updated if the actual voltage ever goes outside of the default range.
pub struct Battery {
    pub ok: bool,
    pub connected: bool,
    pub full: bool,
    pub percent: u8,
    min_voltage: u16,
    max_voltage: u16,
}

impl Battery {
    pub fn new(device: &mut DeviceImpl) -> Result<Self, FSError> {
        let info = ensure_info(device)?;
        let battery = Self {
            ok: false,
            connected: false,
            full: false,
            percent: 50,
            min_voltage: info.min_voltage,
            max_voltage: info.max_voltage,
        };
        Ok(battery)
    }

    pub fn update(&mut self, device: &mut DeviceImpl) -> Result<(), FSError> {
        let Some(status) = device.get_battery_status() else {
            self.ok = false;
            return Ok(());
        };

        self.full = status.full;
        self.connected = status.connected;
        self.ok = true;

        let range = self.max_voltage - self.min_voltage;
        let v_norm = (status.voltage - self.min_voltage) as f32 / range as f32;
        let v_norm = v_norm.clamp(0., 1.);
        let soc = 100. / (1. + exp(-K * (v_norm - 0.5)));
        self.percent = soc as _;

        if status.voltage > self.max_voltage {
            self.max_voltage = status.voltage;
            self.sync_info(device)?;
        }
        if status.voltage < self.min_voltage {
            self.min_voltage = status.voltage;
            self.sync_info(device)?;
        }

        Ok(())
    }

    fn sync_info(&self, device: &mut DeviceImpl) -> Result<(), FSError> {
        let info = BatteryInfo {
            min_voltage: self.min_voltage,
            max_voltage: self.max_voltage,
        };
        let buf = info.encode_vec().unwrap();
        let mut dir = device.open_dir(&["sys"])?;
        let mut file = dir.create_file("battery")?;
        file.write_all(&buf)?;
        Ok(())
    }
}

/// Calculate exponent: e^v.
fn exp(v: f32) -> f32 {
    micromath::F32::from(v).exp().into()
}

fn ensure_info(device: &mut DeviceImpl) -> Result<BatteryInfo, FSError> {
    let mut dir = device.open_dir(&["sys"])?;
    let file = match dir.open_file("battery") {
        Ok(file) => file,
        Err(FSError::NotFound) => return create_info(device),
        Err(err) => return Err(err),
    };
    let raw = read_all(file)?;
    let info = BatteryInfo::decode(&raw).unwrap();
    Ok(info)
}

fn create_info(device: &mut DeviceImpl) -> Result<BatteryInfo, FSError> {
    // Assuming that the battery voltage is in microvolts (mV),
    // the voltage range for a li-ion battery at 25°C is
    // in the range from 3000 mV (3V) to 4200 mV (4.2V).
    let info = BatteryInfo {
        min_voltage: 3_000,
        max_voltage: 4_200,
    };
    let buf = info.encode_vec().unwrap();
    let mut dir = device.open_dir(&["sys"])?;
    let mut file = dir.create_file("battery")?;
    file.write_all(&buf)?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use firefly_hal::*;

    #[test]
    fn test_battery() {
        let mut device = new_device();
        let mut battery = Battery::new(&mut device).ok().unwrap();
        battery.update(&mut device).ok().unwrap();
        assert!(battery.ok);
        let mut dir = device.open_dir(&["sys"]).ok().unwrap();
        dir.open_file("battery").ok().unwrap();
        // TODO(@orsinium): figure out a good way to mock out device voltage.
    }

    fn new_device<'a>() -> DeviceImpl<'a> {
        let root = std::env::temp_dir().join("test_battery");
        _ = std::fs::create_dir_all(root.join("sys"));
        let config = DeviceConfig {
            root,
            ..Default::default()
        };
        DeviceImpl::new(config)
    }
}
