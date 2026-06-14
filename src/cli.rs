use crate::{
    battery::probe_battery,
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
    let snapshot = probe_battery()?;

    println!("Device: {}", snapshot.device_name);

    match snapshot.percentage {
        Some(percentage) => println!("Battery: {percentage}%"),
        None => println!("Battery: unavailable"),
    }

    match snapshot.charging {
        Some(true) => println!("Charging: yes"),
        Some(false) => println!("Charging: no"),
        None => println!("Charging: unknown"),
    }

    Ok(())
}

fn format_optional_hex(value: Option<u16>) -> String {
    value
        .map(|value| format!("0x{value:04X}"))
        .unwrap_or_else(|| "unknown".to_string())
}

fn format_usage(usage: &RazerHidUsage) -> String {
    format!(
        "Interface: {}, UsagePage: {}, Usage: {}",
        usage
            .interface_number
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
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
