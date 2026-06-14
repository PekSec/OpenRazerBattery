#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatterySnapshot {
    pub device_name: String,
    pub vid: u16,
    pub pid: u16,
    pub percentage: Option<u8>,
    pub charging: Option<bool>,
    pub status: BatteryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Ok,
    DeviceNotFound,
    UnsupportedDevice,
    DeviceBusy,
    ProtocolError,
    TransportError,
}

pub fn raw_battery_to_percentage(raw: u8) -> u8 {
    ((u16::from(raw) * 100 + 127) / 255) as u8
}

pub fn unavailable_snapshot(status: BatteryStatus) -> BatterySnapshot {
    BatterySnapshot {
        device_name: "Razer mouse".to_string(),
        vid: 0,
        pid: 0,
        percentage: None,
        charging: None,
        status,
    }
}
