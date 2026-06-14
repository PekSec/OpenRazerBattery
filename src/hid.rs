use crate::{
    device::{RazerHidCandidate, is_razer_vid, known_device},
    error::AppError,
};

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
    candidates.dedup_by(|left, right| {
        left.vid == right.vid
            && left.pid == right.pid
            && left.usage_page == right.usage_page
            && left.usage == right.usage
            && left.name == right.name
    });

    Ok(candidates)
}
