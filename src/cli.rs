use crate::{
    battery::{BatteryStatus, unavailable_snapshot},
    device::{display_name_for_candidate, known_device},
    error::AppError,
    hid,
};

pub fn run_list() -> Result<(), AppError> {
    let candidates = hid::enumerate_razer_hid_candidates()?;

    println!("Razer HID candidates:");
    println!();

    if candidates.is_empty() {
        println!("No Razer HID candidates found.");
        return Ok(());
    }

    for (index, candidate) in candidates.iter().enumerate() {
        let definition = known_device(candidate.vid, candidate.pid);

        println!("[{}]", index + 1);
        println!("Name: {}", display_name_for_candidate(candidate));
        println!("VID: 0x{:04X}", candidate.vid);
        println!("PID: 0x{:04X}", candidate.pid);
        println!("UsagePage: {}", format_optional_hex(candidate.usage_page));
        println!("Usage: {}", format_optional_hex(candidate.usage));
        println!("Known: {}", yes_no(definition.is_some()));
        println!(
            "Battery: {}",
            definition
                .map(|device| support_text(device.supports_battery))
                .unwrap_or("unsupported")
        );
        println!(
            "Charging: {}",
            definition
                .map(|device| support_text(device.supports_charging))
                .unwrap_or("unknown")
        );
        println!();
    }

    Ok(())
}

pub fn run_probe() -> Result<(), AppError> {
    let snapshot = unavailable_snapshot(BatteryStatus::DeviceNotFound);

    println!("Battery probe is not implemented in this scaffold.");
    println!("Status: {:?}", snapshot.status);
    println!("Run `razer-bat.exe list` to show detected Razer HID devices.");

    Ok(())
}

fn format_optional_hex(value: Option<u16>) -> String {
    value
        .map(|value| format!("0x{value:04X}"))
        .unwrap_or_else(|| "unknown".to_string())
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn support_text(value: bool) -> &'static str {
    if value { "supported" } else { "unsupported" }
}
