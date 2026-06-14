use std::{io, thread, time::Duration};

use hidapi::{DeviceInfo, HidApi, HidDevice, HidError};

use crate::{
    device::{RazerHidCandidate, is_razer_vid, known_device},
    error::AppError,
    protocol::{
        RAZER_REPORT_LEN, REPORT_ID, WINDOWS_FEATURE_REPORT_LEN, from_windows_feature_report,
        to_windows_feature_report,
    },
};

pub const FEATURE_RESPONSE_WAIT: Duration = Duration::from_millis(20);

pub struct RazerHidTransport {
    api: HidApi,
}

pub struct RazerHidDevice {
    device: HidDevice,
}

pub fn enumerate_razer_hid_candidates() -> Result<Vec<RazerHidCandidate>, AppError> {
    RazerHidTransport::new()?.enumerate_razer_hid_candidates()
}

pub fn open_candidate(candidate: &RazerHidCandidate) -> Result<RazerHidDevice, AppError> {
    RazerHidTransport::new()?.open_candidate(candidate)
}

impl RazerHidTransport {
    pub fn new() -> Result<Self, AppError> {
        let api = HidApi::new().map_err(|error| map_hid_error(error, HidOperation::Initialize))?;
        Ok(Self { api })
    }

    pub fn enumerate_razer_hid_candidates(&self) -> Result<Vec<RazerHidCandidate>, AppError> {
        let mut candidates = self
            .api
            .device_list()
            .filter(|device| is_razer_vid(device.vendor_id()))
            .map(candidate_from_device_info)
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            left.pid
                .cmp(&right.pid)
                .then(left.interface_number.cmp(&right.interface_number))
                .then(left.usage_page.cmp(&right.usage_page))
                .then(left.usage.cmp(&right.usage))
                .then(left.name.cmp(&right.name))
        });

        Ok(candidates)
    }

    pub fn open_candidate(
        &self,
        candidate: &RazerHidCandidate,
    ) -> Result<RazerHidDevice, AppError> {
        let path = candidate.path.as_deref().ok_or(AppError::HidTransport)?;
        let device = self
            .api
            .open_path(path)
            .map_err(|error| map_hid_error(error, HidOperation::Open))?;

        Ok(RazerHidDevice { device })
    }
}

impl RazerHidDevice {
    pub fn query_feature_report(
        &self,
        request: &[u8; RAZER_REPORT_LEN],
    ) -> Result<[u8; RAZER_REPORT_LEN], AppError> {
        self.send_feature_report(request)?;
        thread::sleep(FEATURE_RESPONSE_WAIT);
        self.receive_feature_report()
    }

    pub fn send_feature_report(&self, request: &[u8; RAZER_REPORT_LEN]) -> Result<(), AppError> {
        let feature_request = to_windows_feature_report(request);
        self.device
            .send_feature_report(&feature_request)
            .map_err(|error| map_hid_error(error, HidOperation::SendFeatureReport))
    }

    pub fn receive_feature_report(&self) -> Result<[u8; RAZER_REPORT_LEN], AppError> {
        let mut feature_response = [0; WINDOWS_FEATURE_REPORT_LEN];
        feature_response[0] = REPORT_ID;

        let read_len = self
            .device
            .get_feature_report(&mut feature_response)
            .map_err(|error| map_hid_error(error, HidOperation::GetFeatureReport))?;

        if read_len != WINDOWS_FEATURE_REPORT_LEN {
            return Err(AppError::InvalidReportLength {
                expected: WINDOWS_FEATURE_REPORT_LEN,
                actual: read_len,
            });
        }

        from_windows_feature_report(&feature_response)
    }
}

fn candidate_from_device_info(device: &DeviceInfo) -> RazerHidCandidate {
    let vid = device.vendor_id();
    let pid = device.product_id();
    let name = device
        .product_string()
        .map(ToOwned::to_owned)
        .or_else(|| known_device(vid, pid).map(|definition| definition.name.to_string()))
        .unwrap_or_else(|| "Unknown Razer HID".to_string());

    RazerHidCandidate {
        name,
        vid,
        pid,
        interface_number: Some(device.interface_number()),
        usage_page: Some(device.usage_page()),
        usage: Some(device.usage()),
        path: Some(device.path().to_owned()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HidOperation {
    Initialize,
    Open,
    SendFeatureReport,
    GetFeatureReport,
}

fn map_hid_error(error: HidError, operation: HidOperation) -> AppError {
    match error {
        HidError::InvalidZeroSizeData => AppError::InvalidReportLength {
            expected: WINDOWS_FEATURE_REPORT_LEN,
            actual: 0,
        },
        HidError::IncompleteSendError { sent, all } => AppError::InvalidReportLength {
            expected: all,
            actual: sent,
        },
        HidError::IoError { error } => map_io_error(&error, operation),
        HidError::OpenHidDeviceWithDeviceInfoError { .. } => AppError::HidTransport,
        HidError::InitializationError
        | HidError::HidApiError { .. }
        | HidError::HidApiErrorEmpty
        | HidError::FromWideCharError { .. }
        | HidError::SetBlockingModeError { .. } => AppError::HidTransport,
    }
}

fn map_io_error(error: &io::Error, operation: HidOperation) -> AppError {
    match error.raw_os_error() {
        Some(2 | 3) if operation == HidOperation::Open => AppError::NoDevice,
        Some(5) => AppError::AccessDenied,
        Some(32 | 33) => AppError::DeviceBusy,
        _ => match error.kind() {
            io::ErrorKind::NotFound if operation == HidOperation::Open => AppError::NoDevice,
            io::ErrorKind::PermissionDenied => AppError::AccessDenied,
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => AppError::DeviceBusy,
            _ => AppError::HidTransport,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_denied_hid_errors_map_to_access_denied() {
        let error = HidError::IoError {
            error: io::Error::from_raw_os_error(5),
        };

        assert_eq!(
            map_hid_error(error, HidOperation::Open),
            AppError::AccessDenied
        );
    }

    #[test]
    fn sharing_violations_map_to_device_busy() {
        let error = HidError::IoError {
            error: io::Error::from_raw_os_error(32),
        };

        assert_eq!(
            map_hid_error(error, HidOperation::Open),
            AppError::DeviceBusy
        );
    }

    #[test]
    fn incomplete_feature_reports_map_to_invalid_length() {
        let error = HidError::IncompleteSendError { sent: 40, all: 91 };

        assert_eq!(
            map_hid_error(error, HidOperation::SendFeatureReport),
            AppError::InvalidReportLength {
                expected: 91,
                actual: 40
            }
        );
    }
}
