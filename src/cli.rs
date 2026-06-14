use crate::{
    battery::{BatteryStatus, unavailable_snapshot},
    error::AppError,
    hid,
};

pub fn run_list() -> Result<(), AppError> {
    let candidates = hid::enumerate_razer_hid_candidates()?;

    println!("Razer HID candidates:");
    println!();

    if candidates.is_empty() {
        println!("HID enumeration is not implemented in this scaffold.");
    }

    Ok(())
}

pub fn run_probe() -> Result<(), AppError> {
    let snapshot = unavailable_snapshot(BatteryStatus::DeviceNotFound);

    println!("Battery probe is not implemented in this scaffold.");
    println!("Status: {:?}", snapshot.status);
    println!("Run `razer-bat.exe list` once HID enumeration is implemented.");

    Ok(())
}
