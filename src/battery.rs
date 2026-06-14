use crate::{
    device::{RazerDeviceDefinition, RazerHidCandidate, display_name_for_candidate, known_device},
    error::AppError,
    hid::RazerHidTransport,
    protocol::{
        build_get_battery_report, build_get_charging_report, parse_battery_raw, parse_charging,
    },
};

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

pub fn probe_battery() -> Result<BatterySnapshot, AppError> {
    let transport = RazerHidTransport::new()?;
    let candidates = transport.enumerate_razer_hid_candidates()?;
    let mut unsupported_razer_seen = false;
    let mut last_error = None;

    for (candidate, definition) in supported_battery_candidates(&candidates) {
        match probe_candidate(&transport, candidate, definition) {
            Ok(snapshot) => return Ok(snapshot),
            Err(error @ AppError::DeviceBusy) => return Err(error),
            Err(error @ AppError::UnsupportedDevice) => last_error = Some(error),
            Err(error @ AppError::AccessDenied) => last_error = Some(error),
            Err(error @ AppError::HidTransport) => last_error = Some(error),
            Err(error @ AppError::InvalidReportLength { .. })
            | Err(error @ AppError::InvalidChecksum)
            | Err(error @ AppError::UnexpectedResponse) => last_error = Some(error),
            Err(error) => last_error = Some(error),
        }
    }

    for candidate in &candidates {
        match known_device(candidate.vid, candidate.pid) {
            Some(definition) if !definition.supports_battery => {
                unsupported_razer_seen = true;
            }
            None => unsupported_razer_seen = true,
            _ => {}
        }
    }

    if let Some(error) = last_error {
        return Err(error);
    }

    if unsupported_razer_seen {
        Err(AppError::UnsupportedDevice)
    } else {
        Err(AppError::NoDevice)
    }
}

fn supported_battery_candidates(
    candidates: &[RazerHidCandidate],
) -> impl Iterator<Item = (&RazerHidCandidate, &'static RazerDeviceDefinition)> {
    candidates.iter().filter_map(|candidate| {
        let definition = known_device(candidate.vid, candidate.pid)?;

        if !definition.supports_battery {
            return None;
        }

        if definition.usage_page.is_some() && definition.usage_page != candidate.usage_page {
            return None;
        }

        if definition.usage.is_some() && definition.usage != candidate.usage {
            return None;
        }

        Some((candidate, definition))
    })
}

fn probe_candidate(
    transport: &RazerHidTransport,
    candidate: &RazerHidCandidate,
    definition: &RazerDeviceDefinition,
) -> Result<BatterySnapshot, AppError> {
    let device = transport.open_candidate(candidate)?;
    let battery_request = build_get_battery_report(definition.transaction_id);
    let battery_response = device.query_feature_report(&battery_request)?;
    let raw_battery = parse_battery_raw(&battery_response)?;

    let charging = if definition.supports_charging {
        let charging_request = build_get_charging_report(definition.transaction_id);
        let charging_response = device.query_feature_report(&charging_request)?;
        Some(parse_charging(&charging_response)?)
    } else {
        Some(false)
    };

    Ok(BatterySnapshot {
        device_name: display_name_for_candidate(candidate).to_string(),
        vid: candidate.vid,
        pid: candidate.pid,
        percentage: Some(raw_battery_to_percentage(raw_battery)),
        charging,
        status: BatteryStatus::Ok,
    })
}
