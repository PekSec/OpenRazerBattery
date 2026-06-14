use crate::{
    battery::{BatteryStatus, unavailable_snapshot},
    device::{RazerHidUsage, known_device, summarize_hid_candidates},
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

    let summaries = summarize_hid_candidates(&candidates);

    for (index, summary) in summaries.iter().enumerate() {
        let definition = known_device(summary.vid, summary.pid);

        println!("[{}]", index + 1);
        println!("Name: {}", summary.name);
        println!("VID: 0x{:04X}", summary.vid);
        println!("PID: 0x{:04X}", summary.pid);
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
        println!("Interfaces: {}", summary.usages.len());

        for usage in &summary.usages {
            println!("  {}", format_usage(usage));
        }

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

fn format_usage(usage: &RazerHidUsage) -> String {
    format!(
        "UsagePage: {}, Usage: {}",
        format_optional_hex(usage.usage_page),
        format_optional_hex(usage.usage)
    )
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn support_text(value: bool) -> &'static str {
    if value { "supported" } else { "unsupported" }
}
