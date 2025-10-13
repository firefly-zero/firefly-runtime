use core::f32::consts::E;

use embedded_io::Write;
use firefly_hal::{Device, DeviceImpl, FSError};
use firefly_types::{BatteryInfo, Encode};

const K: f32 = 12.;

pub struct Battery {
    pub ok: bool,
    pub connected: bool,
    pub full: bool,
    pub percent: u8,
    min_voltage: u16,
    max_voltage: u16,
}

impl Battery {
    pub fn new(device: &mut DeviceImpl) -> Result<Battery, FSError> {
        let info = ensure_info(device)?;
        let battery = Battery {
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

    fn sync_info(&mut self, device: &mut DeviceImpl) -> Result<(), FSError> {
        todo!()
    }
}

fn exp(v: f32) -> f32 {
    // TODO(@orsinium): use micromath
    E.powf(v)
}

fn ensure_info(device: &mut DeviceImpl) -> Result<BatteryInfo, FSError> {
    let file = match device.open_file(&["sys", "battery"]) {
        Ok(file) => file,
        Err(FSError::NotFound) => return create_info(device),
        Err(err) => return Err(err),
    };
    todo!()
}

fn create_info(device: &mut DeviceImpl) -> Result<BatteryInfo, FSError> {
    let info = BatteryInfo {
        min_voltage: 3_000,
        max_voltage: 4_200,
    };
    let buf = info.encode_vec().unwrap();
    let mut file = device.create_file(&["sys", "battery"])?;
    file.write_all(&buf)?;
    Ok(info)
}
