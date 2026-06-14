use crate::{
    device::{RazerHidCandidate, is_razer_vid, known_device},
    error::AppError,
    protocol::{
        RAZER_REPORT_LEN, REPORT_ID, WINDOWS_FEATURE_REPORT_LEN, from_windows_feature_report,
        to_windows_feature_report,
    },
};

pub struct RazerHidDevice {
    device: hidapi::HidDevice,
}

pub fn enumerate_razer_hid_candidates() -> Result<Vec<RazerHidCandidate>, AppError> {
    let api = hidapi::HidApi::new().map_err(|_| AppError::HidTransport)?;
    let mut candidates = api
        .device_list()
        .filter(|device| is_razer_vid(device.vendor_id()))
        .map(|device| {
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
                usage_page: Some(device.usage_page()),
                usage: Some(device.usage()),
                path: Some(device.path().to_owned()),
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.pid
            .cmp(&right.pid)
            .then(left.usage_page.cmp(&right.usage_page))
            .then(left.usage.cmp(&right.usage))
            .then(left.name.cmp(&right.name))
    });

    Ok(candidates)
}

pub fn open_candidate(candidate: &RazerHidCandidate) -> Result<RazerHidDevice, AppError> {
    let api = hidapi::HidApi::new().map_err(|_| AppError::HidTransport)?;
    let path = candidate.path.as_deref().ok_or(AppError::HidTransport)?;
    let device = api.open_path(path).map_err(|_| AppError::AccessDenied)?;

    Ok(RazerHidDevice { device })
}

impl RazerHidDevice {
    pub fn query_feature_report(
        &self,
        request: &[u8; RAZER_REPORT_LEN],
    ) -> Result<[u8; RAZER_REPORT_LEN], AppError> {
        let feature_request = to_windows_feature_report(request);
        self.device
            .send_feature_report(&feature_request)
            .map_err(|_| AppError::HidTransport)?;

        std::thread::sleep(std::time::Duration::from_millis(20));

        let mut feature_response = [0; WINDOWS_FEATURE_REPORT_LEN];
        feature_response[0] = REPORT_ID;
        let read_len = self
            .device
            .get_feature_report(&mut feature_response)
            .map_err(|_| AppError::HidTransport)?;

        if read_len != WINDOWS_FEATURE_REPORT_LEN {
            return Err(AppError::InvalidReportLength {
                expected: WINDOWS_FEATURE_REPORT_LEN,
                actual: read_len,
            });
        }

        from_windows_feature_report(&feature_response)
    }
}
