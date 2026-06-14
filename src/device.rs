pub const RAZER_VID: u16 = 0x1532;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RazerDeviceDefinition {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    pub transaction_id: u8,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub supports_battery: bool,
    pub supports_charging: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RazerHidCandidate {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
}

pub const DEVICES: &[RazerDeviceDefinition] = &[RazerDeviceDefinition {
    vid: RAZER_VID,
    pid: 0x00B7,
    name: "Razer DeathAdder V3 Pro Wireless",
    transaction_id: 0x1F,
    usage_page: Some(0x0001),
    usage: Some(0x0002),
    supports_battery: true,
    supports_charging: true,
}];

pub fn is_razer_vid(vid: u16) -> bool {
    vid == RAZER_VID
}

pub fn known_device(vid: u16, pid: u16) -> Option<&'static RazerDeviceDefinition> {
    DEVICES
        .iter()
        .find(|device| device.vid == vid && device.pid == pid)
}

pub fn select_supported_device(
    candidates: &[RazerHidCandidate],
) -> Option<(&RazerHidCandidate, &'static RazerDeviceDefinition)> {
    candidates.iter().find_map(|candidate| {
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
