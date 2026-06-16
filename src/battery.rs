use std::collections::BTreeSet;

use crate::{
    device::{
        RazerDeviceDefinition, RazerHidCandidate, display_name_for_candidate, known_device,
        ranked_supported_battery_candidates,
    },
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
    AccessDenied,
    DeviceBusy,
    ProtocolError,
    TransportError,
}

impl BatteryStatus {
    pub fn user_message(self) -> &'static str {
        match self {
            Self::Ok => "Battery available.",
            Self::DeviceNotFound => "No supported Razer mouse found.",
            Self::UnsupportedDevice => "Unsupported Razer device.",
            Self::AccessDenied => "Device access denied.",
            Self::DeviceBusy => "Device busy. Will retry.",
            Self::ProtocolError => "Battery query failed.",
            Self::TransportError => "HID transport failed.",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::DeviceNotFound => "not found",
            Self::UnsupportedDevice => "unsupported",
            Self::AccessDenied => "access denied",
            Self::DeviceBusy => "busy",
            Self::ProtocolError => "query failed",
            Self::TransportError => "HID failed",
        }
    }
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
    let snapshots = probe_batteries();

    if let Some(snapshot) = snapshots
        .iter()
        .find(|snapshot| snapshot.status == BatteryStatus::Ok)
    {
        return Ok(snapshot.clone());
    }

    let status = snapshots
        .first()
        .map(|snapshot| snapshot.status)
        .unwrap_or(BatteryStatus::DeviceNotFound);

    Err(app_error_for_status(status))
}

pub fn probe_batteries() -> Vec<BatterySnapshot> {
    match probe_batteries_inner() {
        Ok(snapshots) => snapshots,
        Err(error) => vec![unavailable_snapshot(battery_status_for_error(&error))],
    }
}

fn probe_batteries_inner() -> Result<Vec<BatterySnapshot>, AppError> {
    let transport = RazerHidTransport::new()?;
    let candidates = transport.enumerate_razer_hid_candidates()?;
    let mut unsupported_razer_seen = false;
    let ranked_candidates = ranked_supported_battery_candidates(&candidates);

    if !ranked_candidates.is_empty() {
        let mut seen_devices = BTreeSet::new();
        let mut snapshots = Vec::new();

        for (candidate, definition) in &ranked_candidates {
            if !seen_devices.insert((candidate.vid, candidate.pid)) {
                continue;
            }

            snapshots.push(probe_ranked_device_candidates(
                &transport,
                &ranked_candidates,
                definition,
            ));
        }

        return Ok(snapshots);
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

    if unsupported_razer_seen {
        Ok(vec![unavailable_snapshot(BatteryStatus::UnsupportedDevice)])
    } else {
        Ok(vec![unavailable_snapshot(BatteryStatus::DeviceNotFound)])
    }
}

fn probe_ranked_device_candidates(
    transport: &RazerHidTransport,
    ranked_candidates: &[(&RazerHidCandidate, &RazerDeviceDefinition)],
    definition: &RazerDeviceDefinition,
) -> BatterySnapshot {
    let mut first_failure = None;

    for (candidate, candidate_definition) in ranked_candidates {
        if candidate_definition.vid != definition.vid || candidate_definition.pid != definition.pid
        {
            continue;
        }

        match probe_candidate(transport, candidate, candidate_definition) {
            Ok(snapshot) => return snapshot,
            Err(error) => {
                first_failure.get_or_insert((*candidate, battery_status_for_error(&error)));
            }
        }
    }

    first_failure
        .map(|(candidate, status)| unavailable_candidate_snapshot(candidate, status))
        .unwrap_or_else(|| unavailable_snapshot(BatteryStatus::UnsupportedDevice))
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

fn unavailable_candidate_snapshot(
    candidate: &RazerHidCandidate,
    status: BatteryStatus,
) -> BatterySnapshot {
    BatterySnapshot {
        device_name: display_name_for_candidate(candidate).to_string(),
        vid: candidate.vid,
        pid: candidate.pid,
        percentage: None,
        charging: None,
        status,
    }
}

fn battery_status_for_error(error: &AppError) -> BatteryStatus {
    match error {
        AppError::NoDevice => BatteryStatus::DeviceNotFound,
        AppError::UnsupportedDevice => BatteryStatus::UnsupportedDevice,
        AppError::AccessDenied => BatteryStatus::AccessDenied,
        AppError::DeviceBusy => BatteryStatus::DeviceBusy,
        AppError::InvalidReportLength { .. }
        | AppError::InvalidChecksum
        | AppError::UnexpectedResponse => BatteryStatus::ProtocolError,
        AppError::HidTransport => BatteryStatus::TransportError,
        AppError::InvalidCommand | AppError::Tray => BatteryStatus::TransportError,
    }
}

fn app_error_for_status(status: BatteryStatus) -> AppError {
    match status {
        BatteryStatus::Ok => AppError::UnexpectedResponse,
        BatteryStatus::DeviceNotFound => AppError::NoDevice,
        BatteryStatus::UnsupportedDevice => AppError::UnsupportedDevice,
        BatteryStatus::AccessDenied => AppError::AccessDenied,
        BatteryStatus::DeviceBusy => AppError::DeviceBusy,
        BatteryStatus::ProtocolError => AppError::UnexpectedResponse,
        BatteryStatus::TransportError => AppError::HidTransport,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_errors_map_to_battery_statuses() {
        assert_eq!(
            battery_status_for_error(&AppError::NoDevice),
            BatteryStatus::DeviceNotFound
        );
        assert_eq!(
            battery_status_for_error(&AppError::UnsupportedDevice),
            BatteryStatus::UnsupportedDevice
        );
        assert_eq!(
            battery_status_for_error(&AppError::AccessDenied),
            BatteryStatus::AccessDenied
        );
        assert_eq!(
            battery_status_for_error(&AppError::DeviceBusy),
            BatteryStatus::DeviceBusy
        );
        assert_eq!(
            battery_status_for_error(&AppError::InvalidChecksum),
            BatteryStatus::ProtocolError
        );
        assert_eq!(
            battery_status_for_error(&AppError::HidTransport),
            BatteryStatus::TransportError
        );
    }
}
