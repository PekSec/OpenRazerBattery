pub const RAZER_VID: u16 = 0x1532;
pub const MOUSE_USAGE_PAGE: u16 = 0x0001;
pub const MOUSE_USAGE: u16 = 0x0002;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RazerHidUsage {
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RazerHidDeviceSummary {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub usages: Vec<RazerHidUsage>,
}

const fn mouse(pid: u16, name: &'static str) -> RazerDeviceDefinition {
    RazerDeviceDefinition {
        vid: RAZER_VID,
        pid,
        name,
        transaction_id: 0xFF,
        usage_page: Some(MOUSE_USAGE_PAGE),
        usage: Some(MOUSE_USAGE),
        supports_battery: false,
        supports_charging: false,
    }
}

const fn battery_mouse(
    pid: u16,
    name: &'static str,
    transaction_id: u8,
    supports_charging: bool,
) -> RazerDeviceDefinition {
    RazerDeviceDefinition {
        vid: RAZER_VID,
        pid,
        name,
        transaction_id,
        usage_page: Some(MOUSE_USAGE_PAGE),
        usage: Some(MOUSE_USAGE),
        supports_battery: true,
        supports_charging,
    }
}

// Mouse VID/PID names come from OpenRazer's public supported-device list.
// Battery capability and transaction IDs come from OpenRazer's mouse driver
// charge_level/charge_status handling. Keep this as the single device catalog.
pub const DEVICES: &[RazerDeviceDefinition] = &[
    mouse(0x0013, "Razer Orochi 2011"),
    mouse(0x0015, "Razer Naga"),
    mouse(0x0016, "Razer DeathAdder 3.5G"),
    battery_mouse(0x001F, "Razer Naga Epic", 0xFF, true),
    mouse(0x0020, "Razer Abyssus 1800"),
    battery_mouse(0x0024, "Razer Mamba 2012 (Wired)", 0xFF, true),
    battery_mouse(0x0025, "Razer Mamba 2012 (Wireless)", 0xFF, true),
    mouse(0x0029, "Razer DeathAdder 3.5G Black"),
    mouse(0x002E, "Razer Naga 2012"),
    mouse(0x002F, "Razer Imperator 2012"),
    battery_mouse(0x0032, "Razer Ouroboros 2012", 0xFF, true),
    mouse(0x0034, "Razer Taipan"),
    mouse(0x0036, "Razer Naga Hex (Red)"),
    mouse(0x0037, "Razer DeathAdder 2013"),
    mouse(0x0038, "Razer DeathAdder 1800"),
    mouse(0x0039, "Razer Orochi 2013"),
    battery_mouse(0x003E, "Razer Naga Epic Chroma (Wired)", 0xFF, true),
    battery_mouse(0x003F, "Razer Naga Epic Chroma (Wireless)", 0xFF, true),
    mouse(0x0040, "Razer Naga 2014"),
    mouse(0x0041, "Razer Naga Hex"),
    mouse(0x0042, "Razer Abyssus 2014"),
    mouse(0x0043, "Razer DeathAdder Chroma"),
    battery_mouse(0x0044, "Razer Mamba (Wired)", 0xFF, true),
    battery_mouse(0x0045, "Razer Mamba (Wireless)", 0xFF, true),
    mouse(0x0046, "Razer Mamba Tournament Edition"),
    mouse(0x0048, "Razer Orochi (Wired)"),
    mouse(0x004C, "Razer Diamondback Chroma"),
    mouse(0x004F, "Razer DeathAdder 2000"),
    mouse(0x0050, "Razer Naga Hex V2"),
    mouse(0x0053, "Razer Naga Chroma"),
    mouse(0x0054, "Razer DeathAdder 3500"),
    battery_mouse(0x0059, "Razer Lancehead (Wired)", 0x3F, true),
    battery_mouse(0x005A, "Razer Lancehead (Wireless)", 0x3F, true),
    mouse(0x005B, "Razer Abyssus V2"),
    mouse(0x005C, "Razer DeathAdder Elite"),
    mouse(0x005E, "Razer Abyssus 2000"),
    mouse(0x0060, "Razer Lancehead Tournament Edition"),
    battery_mouse(0x0062, "Razer Atheris (Receiver)", 0x1F, false),
    mouse(0x0064, "Razer Basilisk"),
    mouse(0x0065, "Razer Basilisk Essential"),
    mouse(0x0067, "Razer Naga Trinity"),
    mouse(0x006A, "Razer Abyssus Elite (D.Va Edition)"),
    mouse(0x006B, "Razer Abyssus Essential"),
    mouse(0x006C, "Razer Mamba Elite (Wired)"),
    mouse(0x006E, "Razer DeathAdder Essential"),
    battery_mouse(0x006F, "Razer Lancehead Wireless (Receiver)", 0x1F, true),
    battery_mouse(0x0070, "Razer Lancehead Wireless (Wired)", 0x1F, true),
    mouse(0x0071, "Razer DeathAdder Essential (White Edition)"),
    battery_mouse(0x0072, "Razer Mamba Wireless (Receiver)", 0x3F, true),
    battery_mouse(0x0073, "Razer Mamba Wireless (Wired)", 0x3F, true),
    battery_mouse(0x0077, "Razer Pro Click (Receiver)", 0x1F, true),
    mouse(0x0078, "Razer Viper"),
    battery_mouse(0x007A, "Razer Viper Ultimate (Wired)", 0xFF, true),
    battery_mouse(0x007B, "Razer Viper Ultimate (Wireless)", 0xFF, true),
    battery_mouse(0x007C, "Razer DeathAdder V2 Pro (Wired)", 0x3F, true),
    battery_mouse(0x007D, "Razer DeathAdder V2 Pro (Wireless)", 0x3F, true),
    battery_mouse(0x0080, "Razer Pro Click (Wired)", 0x1F, true),
    battery_mouse(0x0083, "Razer Basilisk X HyperSpeed", 0xFF, false),
    mouse(0x0084, "Razer DeathAdder V2"),
    mouse(0x0085, "Razer Basilisk V2"),
    battery_mouse(0x0086, "Razer Basilisk Ultimate (Wired)", 0x1F, true),
    battery_mouse(0x0088, "Razer Basilisk Ultimate (Receiver)", 0x1F, true),
    mouse(0x008A, "Razer Viper Mini"),
    mouse(0x008C, "Razer DeathAdder V2 Mini"),
    mouse(0x008D, "Razer Naga Left-Handed Edition"),
    battery_mouse(0x008F, "Razer Naga Pro (Wired)", 0x1F, true),
    battery_mouse(0x0090, "Razer Naga Pro (Wireless)", 0x1F, true),
    mouse(0x0091, "Razer Viper 8KHz"),
    battery_mouse(0x0094, "Razer Orochi V2 (Receiver)", 0x1F, false),
    battery_mouse(0x0095, "Razer Orochi V2 (Bluetooth)", 0x1F, false),
    mouse(0x0096, "Razer Naga X"),
    mouse(0x0098, "Razer DeathAdder Essential (2021)"),
    mouse(0x0099, "Razer Basilisk V3"),
    battery_mouse(0x009A, "Razer Pro Click Mini (Receiver)", 0x1F, true),
    battery_mouse(0x009C, "Razer DeathAdder V2 X HyperSpeed", 0x1F, false),
    battery_mouse(
        0x009E,
        "Razer Viper Mini Signature Edition (Wired)",
        0x1F,
        true,
    ),
    battery_mouse(
        0x009F,
        "Razer Viper Mini Signature Edition (Wireless)",
        0x1F,
        true,
    ),
    mouse(0x00A1, "Razer DeathAdder V2 Lite"),
    mouse(0x00A3, "Razer Cobra"),
    battery_mouse(0x00A5, "Razer Viper V2 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00A6, "Razer Viper V2 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00A7, "Razer Naga V2 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00A8, "Razer Naga V2 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00AA, "Razer Basilisk V3 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00AB, "Razer Basilisk V3 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00AF, "Razer Cobra Pro (Wired)", 0x1F, true),
    battery_mouse(0x00B0, "Razer Cobra Pro (Wireless)", 0x1F, true),
    mouse(0x00B2, "Razer DeathAdder V3"),
    battery_mouse(0x00B3, "Razer HyperPolling Wireless Dongle", 0x1F, true),
    battery_mouse(0x00B4, "Razer Naga V2 HyperSpeed (Receiver)", 0x1F, false),
    battery_mouse(0x00B6, "Razer DeathAdder V3 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00B7, "Razer DeathAdder V3 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00B8, "Razer Viper V3 HyperSpeed", 0x1F, false),
    battery_mouse(0x00B9, "Razer Basilisk V3 X HyperSpeed", 0x1F, false),
    battery_mouse(0x00BE, "Razer DeathAdder V4 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00BF, "Razer DeathAdder V4 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00C0, "Razer Viper V3 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00C1, "Razer Viper V3 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00C2, "Razer DeathAdder V3 Pro (Wired)", 0x1F, true),
    battery_mouse(0x00C3, "Razer DeathAdder V3 Pro (Wireless)", 0x1F, true),
    battery_mouse(0x00C4, "Razer DeathAdder V3 HyperSpeed (Wired)", 0x1F, true),
    battery_mouse(
        0x00C5,
        "Razer DeathAdder V3 HyperSpeed (Wireless)",
        0x1F,
        true,
    ),
    battery_mouse(
        0x00C7,
        "Razer Pro Click V2 Vertical Edition (Wired)",
        0x1F,
        true,
    ),
    battery_mouse(
        0x00C8,
        "Razer Pro Click V2 Vertical Edition (Wireless)",
        0x1F,
        true,
    ),
    battery_mouse(0x00CB, "Razer Basilisk V3 35K", 0x1F, false),
    battery_mouse(0x00CC, "Razer Basilisk V3 Pro 35K (Wired)", 0x1F, true),
    battery_mouse(0x00CD, "Razer Basilisk V3 Pro 35K (Wireless)", 0x1F, true),
    battery_mouse(0x00D0, "Razer Pro Click V2 (Wired)", 0x1F, true),
    battery_mouse(0x00D1, "Razer Pro Click V2 (Wireless)", 0x1F, true),
    battery_mouse(0x00D3, "Razer Basilisk Mobile (Wired)", 0x1F, false),
    battery_mouse(0x00D4, "Razer Basilisk Mobile (Receiver)", 0x1F, false),
    battery_mouse(
        0x00D6,
        "Razer Basilisk V3 Pro 35K Phantom Green Edition (Wired)",
        0x1F,
        true,
    ),
    battery_mouse(
        0x00D7,
        "Razer Basilisk V3 Pro 35K Phantom Green Edition (Wireless)",
        0x1F,
        true,
    ),
];

pub fn is_razer_vid(vid: u16) -> bool {
    vid == RAZER_VID
}

pub fn known_device(vid: u16, pid: u16) -> Option<&'static RazerDeviceDefinition> {
    DEVICES
        .iter()
        .find(|device| device.vid == vid && device.pid == pid)
}

pub fn display_name_for_candidate(candidate: &RazerHidCandidate) -> &str {
    known_device(candidate.vid, candidate.pid)
        .map(|device| device.name)
        .unwrap_or(candidate.name.as_str())
}

pub fn summarize_hid_candidates(candidates: &[RazerHidCandidate]) -> Vec<RazerHidDeviceSummary> {
    let mut summaries = Vec::new();

    for candidate in candidates {
        let name = display_name_for_candidate(candidate).to_string();
        let usage = RazerHidUsage {
            usage_page: candidate.usage_page,
            usage: candidate.usage,
        };

        if let Some(summary) = summaries
            .iter_mut()
            .find(|summary: &&mut RazerHidDeviceSummary| {
                summary.vid == candidate.vid && summary.pid == candidate.pid && summary.name == name
            })
        {
            summary.usages.push(usage);
        } else {
            summaries.push(RazerHidDeviceSummary {
                name,
                vid: candidate.vid,
                pid: candidate.pid,
                usages: vec![usage],
            });
        }
    }

    for summary in &mut summaries {
        summary.usages.sort();
        summary.usages.dedup();
    }

    summaries.sort_by(|left, right| {
        left.pid
            .cmp(&right.pid)
            .then(left.name.cmp(&right.name))
            .then(left.vid.cmp(&right.vid))
    });
    summaries
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
